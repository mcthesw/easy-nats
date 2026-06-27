use std::collections::BTreeMap;

use crate::proto::{AutoDetectResult, ProtoSchemaManager};
use rmpv::{Value, encode::write_value};

use super::{
    BindingResolution, MessageSchemaManager, OutgoingPayload, PayloadInputFormat,
    PayloadSchemaStatus, RenderedSchemaPayload, SchemaBinding, SchemaSelector, SchemaStatusLevel,
    ValidationPolicy,
};

impl MessageSchemaManager {
    #[allow(dead_code)]
    pub fn prepare_outgoing(
        &self,
        connection_id: u64,
        subject: &str,
        payload_text: &str,
    ) -> OutgoingPayload {
        self.prepare_outgoing_with_input_format(
            connection_id,
            subject,
            payload_text,
            PayloadInputFormat::Text,
        )
    }

    pub fn prepare_outgoing_with_input_format(
        &self,
        connection_id: u64,
        subject: &str,
        payload_text: &str,
        input_format: PayloadInputFormat,
    ) -> OutgoingPayload {
        match self.resolve_binding(connection_id, subject) {
            BindingResolution::NoMatch => prepare_without_binding(payload_text, input_format),
            BindingResolution::Ambiguous(bindings) => {
                let status = PayloadSchemaStatus {
                    level: SchemaStatusLevel::Warning,
                    label: "Ambiguous schema binding".to_string(),
                    message: format!("{} bindings match this subject", bindings.len()),
                    can_send: true,
                };
                OutgoingPayload {
                    payload: payload_text.as_bytes().to_vec(),
                    status: Some(status),
                    can_send: true,
                }
            }
            BindingResolution::Match(binding) => self.prepare_with_binding(binding, payload_text),
        }
    }

    pub fn render_schema_payload(
        &self,
        connection_id: u64,
        subject: &str,
        payload: &[u8],
    ) -> Option<RenderedSchemaPayload> {
        match self.resolve_binding(connection_id, subject) {
            BindingResolution::NoMatch => None,
            BindingResolution::Ambiguous(bindings) => Some(RenderedSchemaPayload {
                status: PayloadSchemaStatus {
                    level: SchemaStatusLevel::Warning,
                    label: "Ambiguous schema binding".to_string(),
                    message: format!("{} bindings match this subject", bindings.len()),
                    can_send: true,
                },
                json: None,
            }),
            BindingResolution::Match(binding) => Some(self.render_with_binding(binding, payload)),
        }
    }

    pub fn payload_template(
        &self,
        connection_id: u64,
        subject: &str,
    ) -> Result<Option<String>, String> {
        if subject.trim().is_empty() {
            return Ok(None);
        }
        match self.resolve_binding(connection_id, subject) {
            BindingResolution::NoMatch => Ok(None),
            BindingResolution::Ambiguous(bindings) => {
                Err(format!("{} bindings match this subject", bindings.len()))
            }
            BindingResolution::Match(binding) => self.template_with_binding(binding).map(Some),
        }
    }

    pub fn manual_message_types(&self) -> Vec<String> {
        let mut types = BTreeMap::new();
        for manager in self.proto_sources.values() {
            for ty in manager.message_types() {
                types.insert(ty.clone(), ());
            }
        }
        types.into_keys().collect()
    }

    pub fn decode_manual_proto(&self, data: &[u8], message_type: &str) -> Result<String, String> {
        let mut last_error = None;
        for source_id in self.proto_source_ids() {
            let Some(manager) = self.proto_sources.get(&source_id) else {
                continue;
            };
            if manager.message_types().iter().any(|ty| ty == message_type) {
                match manager.decode(data, message_type) {
                    Ok(json) => return Ok(json),
                    Err(error) => last_error = Some(error),
                }
            }
        }
        Err(last_error.unwrap_or_else(|| format!("Unknown message type: {message_type}")))
    }

    fn proto_source_ids(&self) -> Vec<u64> {
        self.proto_sources.keys().copied().collect()
    }

    pub fn auto_detect_manual_proto(&self, data: &[u8]) -> AutoDetectResult {
        let mut matches = Vec::new();
        for manager in self.proto_sources.values() {
            match manager.auto_detect_and_decode(data) {
                AutoDetectResult::Match { type_name, json } => matches.push((type_name, json)),
                AutoDetectResult::Ambiguous(types) => {
                    return AutoDetectResult::Ambiguous(types);
                }
                AutoDetectResult::NoMatch => {}
            }
        }
        match matches.len() {
            0 => AutoDetectResult::NoMatch,
            1 => {
                let (type_name, json) = matches.into_iter().next().unwrap();
                AutoDetectResult::Match { type_name, json }
            }
            _ => AutoDetectResult::Ambiguous(
                matches
                    .into_iter()
                    .map(|(type_name, _)| type_name)
                    .collect(),
            ),
        }
    }

    pub fn decode_wire_format(data: &[u8]) -> String {
        ProtoSchemaManager::decode_wire_format(data)
    }

    fn prepare_with_binding(&self, binding: &SchemaBinding, payload_text: &str) -> OutgoingPayload {
        match &binding.selector {
            SchemaSelector::ProtobufMessage { type_name } => {
                let Some(manager) = self.proto_sources.get(&binding.source_id) else {
                    return self.unavailable_outgoing(binding, payload_text);
                };
                match manager.encode_json(payload_text, type_name) {
                    Ok(payload) => {
                        let status = PayloadSchemaStatus {
                            level: SchemaStatusLevel::Success,
                            label: self.binding_label(binding),
                            message: "Valid Protobuf payload".to_string(),
                            can_send: true,
                        };
                        OutgoingPayload {
                            payload,
                            status: Some(status),
                            can_send: true,
                        }
                    }
                    Err(error) => self.invalid_outgoing(binding, payload_text, error),
                }
            }
            SchemaSelector::JsonSchema { entry } => {
                let Some(catalog) = self.json_sources.get(&binding.source_id) else {
                    return self.unavailable_outgoing(binding, payload_text);
                };
                let json = match serde_json::from_str::<serde_json::Value>(payload_text) {
                    Ok(json) => json,
                    Err(error) => {
                        return self.invalid_outgoing(
                            binding,
                            payload_text,
                            format!("Invalid JSON: {error}"),
                        );
                    }
                };
                match catalog.validate(entry, &json) {
                    Ok(()) => {
                        let status = PayloadSchemaStatus {
                            level: SchemaStatusLevel::Success,
                            label: self.binding_label(binding),
                            message: "Valid JSON Schema payload".to_string(),
                            can_send: true,
                        };
                        OutgoingPayload {
                            payload: payload_text.as_bytes().to_vec(),
                            status: Some(status),
                            can_send: true,
                        }
                    }
                    Err(error) => self.invalid_outgoing(binding, payload_text, error),
                }
            }
        }
    }

    fn render_with_binding(
        &self,
        binding: &SchemaBinding,
        payload: &[u8],
    ) -> RenderedSchemaPayload {
        match &binding.selector {
            SchemaSelector::ProtobufMessage { type_name } => {
                let Some(manager) = self.proto_sources.get(&binding.source_id) else {
                    return RenderedSchemaPayload {
                        status: self.unavailable_status(binding),
                        json: None,
                    };
                };
                match manager.decode(payload, type_name) {
                    Ok(json) => RenderedSchemaPayload {
                        status: PayloadSchemaStatus {
                            level: SchemaStatusLevel::Success,
                            label: self.binding_label(binding),
                            message: "Decoded with Protobuf schema".to_string(),
                            can_send: true,
                        },
                        json: Some(json),
                    },
                    Err(error) => RenderedSchemaPayload {
                        status: PayloadSchemaStatus {
                            level: SchemaStatusLevel::Error,
                            label: self.binding_label(binding),
                            message: error,
                            can_send: true,
                        },
                        json: None,
                    },
                }
            }
            SchemaSelector::JsonSchema { entry } => {
                let Some(catalog) = self.json_sources.get(&binding.source_id) else {
                    return RenderedSchemaPayload {
                        status: self.unavailable_status(binding),
                        json: None,
                    };
                };
                let json = match serde_json::from_slice::<serde_json::Value>(payload) {
                    Ok(json) => json,
                    Err(error) => {
                        return RenderedSchemaPayload {
                            status: PayloadSchemaStatus {
                                level: SchemaStatusLevel::Error,
                                label: self.binding_label(binding),
                                message: format!("Invalid JSON: {error}"),
                                can_send: true,
                            },
                            json: None,
                        };
                    }
                };
                let pretty =
                    serde_json::to_string_pretty(&json).unwrap_or_else(|_| json.to_string());
                match catalog.validate(entry, &json) {
                    Ok(()) => RenderedSchemaPayload {
                        status: PayloadSchemaStatus {
                            level: SchemaStatusLevel::Success,
                            label: self.binding_label(binding),
                            message: "Valid JSON Schema payload".to_string(),
                            can_send: true,
                        },
                        json: Some(pretty),
                    },
                    Err(error) => RenderedSchemaPayload {
                        status: PayloadSchemaStatus {
                            level: SchemaStatusLevel::Error,
                            label: self.binding_label(binding),
                            message: error,
                            can_send: true,
                        },
                        json: Some(pretty),
                    },
                }
            }
        }
    }

    fn template_with_binding(&self, binding: &SchemaBinding) -> Result<String, String> {
        match &binding.selector {
            SchemaSelector::ProtobufMessage { type_name } => {
                let Some(manager) = self.proto_sources.get(&binding.source_id) else {
                    return Err("Schema source is unavailable".to_string());
                };
                manager.json_template(type_name)
            }
            SchemaSelector::JsonSchema { entry } => {
                let Some(catalog) = self.json_sources.get(&binding.source_id) else {
                    return Err("Schema source is unavailable".to_string());
                };
                catalog.template(entry)
            }
        }
    }

    fn unavailable_outgoing(&self, binding: &SchemaBinding, payload_text: &str) -> OutgoingPayload {
        let status = self.unavailable_status(binding);
        let can_send = status.can_send;
        OutgoingPayload {
            payload: payload_text.as_bytes().to_vec(),
            status: Some(status),
            can_send,
        }
    }

    fn unavailable_status(&self, binding: &SchemaBinding) -> PayloadSchemaStatus {
        let can_send = binding.policy != ValidationPolicy::Block;
        PayloadSchemaStatus {
            level: if can_send {
                SchemaStatusLevel::Warning
            } else {
                SchemaStatusLevel::Error
            },
            label: self.binding_label(binding),
            message: "Schema source is unavailable".to_string(),
            can_send,
        }
    }

    fn invalid_outgoing(
        &self,
        binding: &SchemaBinding,
        payload_text: &str,
        error: String,
    ) -> OutgoingPayload {
        let can_send = binding.policy != ValidationPolicy::Block;
        let level = match binding.policy {
            ValidationPolicy::Inspect => SchemaStatusLevel::Info,
            ValidationPolicy::Warn => SchemaStatusLevel::Warning,
            ValidationPolicy::Block => SchemaStatusLevel::Error,
        };
        let status = PayloadSchemaStatus {
            level,
            label: self.binding_label(binding),
            message: error,
            can_send,
        };
        OutgoingPayload {
            payload: payload_text.as_bytes().to_vec(),
            status: Some(status),
            can_send,
        }
    }

    fn binding_label(&self, binding: &SchemaBinding) -> String {
        let source_name = self
            .config
            .sources
            .iter()
            .find(|source| source.id == binding.source_id)
            .map(|source| source.name.as_str())
            .unwrap_or("Unknown source");
        format!("{source_name} / {}", binding.selector.entry())
    }
}

fn prepare_without_binding(
    payload_text: &str,
    input_format: PayloadInputFormat,
) -> OutgoingPayload {
    match input_format {
        PayloadInputFormat::Text => OutgoingPayload {
            payload: payload_text.as_bytes().to_vec(),
            status: None,
            can_send: true,
        },
        PayloadInputFormat::MessagePack => match encode_messagepack_json(payload_text) {
            Ok(payload) => OutgoingPayload {
                payload,
                status: Some(PayloadSchemaStatus {
                    level: SchemaStatusLevel::Info,
                    label: "MsgPack input".to_string(),
                    message: "JSON will be encoded as MsgPack".to_string(),
                    can_send: true,
                }),
                can_send: true,
            },
            Err(error) => OutgoingPayload {
                payload: Vec::new(),
                status: Some(PayloadSchemaStatus {
                    level: SchemaStatusLevel::Error,
                    label: "MsgPack input".to_string(),
                    message: error,
                    can_send: false,
                }),
                can_send: false,
            },
        },
    }
}

fn encode_messagepack_json(input: &str) -> Result<Vec<u8>, String> {
    let json = serde_json::from_str::<serde_json::Value>(input)
        .map_err(|error| format!("Invalid JSON for MsgPack input: {error}"))?;
    let value = json_to_msgpack(json)?;
    let mut out = Vec::new();
    write_value(&mut out, &value).map_err(|error| format!("MsgPack encode error: {error}"))?;
    Ok(out)
}

fn json_to_msgpack(value: serde_json::Value) -> Result<Value, String> {
    Ok(match value {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(value) => Value::Boolean(value),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Value::from(value)
            } else if let Some(value) = value.as_u64() {
                Value::from(value)
            } else if let Some(value) = value.as_f64() {
                Value::F64(value)
            } else {
                return Err("Unsupported JSON number".to_string());
            }
        }
        serde_json::Value::String(value) => Value::String(value.into()),
        serde_json::Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(json_to_msgpack)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        serde_json::Value::Object(values) => Value::Map(
            values
                .into_iter()
                .map(|(key, value)| Ok((Value::String(key.into()), json_to_msgpack(value)?)))
                .collect::<Result<Vec<_>, String>>()?,
        ),
    })
}
