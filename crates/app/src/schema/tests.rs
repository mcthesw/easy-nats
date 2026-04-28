use super::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::proto::ProtoSchemaManager;

use super::json_schema::{JsonSchemaCatalog, json_schema_template, schema_entry_name};

#[test]
fn subject_pattern_matches_nats_wildcards() {
    let exact = SubjectPattern::parse("orders.created").unwrap();
    assert!(exact.matches("orders.created"));
    assert!(!exact.matches("orders.updated"));

    let one = SubjectPattern::parse("orders.*").unwrap();
    assert!(one.matches("orders.created"));
    assert!(!one.matches("orders.us.created"));

    let tail = SubjectPattern::parse("orders.>").unwrap();
    assert!(tail.matches("orders.created"));
    assert!(tail.matches("orders.us.created"));
    assert!(!tail.matches("orders"));
}

#[test]
fn subject_pattern_rejects_invalid_tail_position() {
    assert!(SubjectPattern::parse("orders.>.created").is_err());
    assert!(SubjectPattern::parse("orders..created").is_err());
    assert!(SubjectPattern::parse("orders.created").is_ok());
}

#[test]
fn resolver_prefers_connection_specific_and_specific_subjects() {
    let mut config = MessageSchemaConfig::default();
    let source_id = config.add_source(
        "schemas".to_string(),
        SchemaSourceKind::JsonSchema,
        "unused".to_string(),
    );
    config
        .add_binding(
            "all".to_string(),
            None,
            "orders.>".to_string(),
            source_id,
            SchemaSelector::JsonSchema {
                entry: "order".to_string(),
            },
            ValidationPolicy::Inspect,
        )
        .unwrap();
    let specific = config
        .add_binding(
            "specific".to_string(),
            Some(7),
            "orders.created".to_string(),
            source_id,
            SchemaSelector::JsonSchema {
                entry: "created".to_string(),
            },
            ValidationPolicy::Block,
        )
        .unwrap();
    let manager = MessageSchemaManager::from_config(config);

    match manager.resolve_binding(7, "orders.created") {
        BindingResolution::Match(binding) => assert_eq!(binding.id, specific),
        _ => panic!("expected matching binding"),
    }
}

#[test]
fn resolver_uses_more_specific_subject_for_same_scope() {
    let mut config = MessageSchemaConfig::default();
    let source_id = config.add_source(
        "schemas".to_string(),
        SchemaSourceKind::JsonSchema,
        "unused".to_string(),
    );
    config
        .add_binding(
            "tail".to_string(),
            None,
            "orders.>".to_string(),
            source_id,
            SchemaSelector::JsonSchema {
                entry: "tail".to_string(),
            },
            ValidationPolicy::Inspect,
        )
        .unwrap();
    let exact = config
        .add_binding(
            "exact".to_string(),
            None,
            "orders.created".to_string(),
            source_id,
            SchemaSelector::JsonSchema {
                entry: "exact".to_string(),
            },
            ValidationPolicy::Inspect,
        )
        .unwrap();
    let manager = MessageSchemaManager::from_config(config);

    match manager.resolve_binding(1, "orders.created") {
        BindingResolution::Match(binding) => assert_eq!(binding.id, exact),
        _ => panic!("expected exact binding"),
    }
}

#[test]
fn resolver_reports_ambiguous_same_rank_and_order() {
    let mut config = MessageSchemaConfig::default();
    let source_id = config.add_source(
        "schemas".to_string(),
        SchemaSourceKind::JsonSchema,
        "unused".to_string(),
    );
    let first = config
        .add_binding(
            "first".to_string(),
            None,
            "orders.*".to_string(),
            source_id,
            SchemaSelector::JsonSchema {
                entry: "first".to_string(),
            },
            ValidationPolicy::Inspect,
        )
        .unwrap();
    let second = config
        .add_binding(
            "second".to_string(),
            None,
            "orders.*".to_string(),
            source_id,
            SchemaSelector::JsonSchema {
                entry: "second".to_string(),
            },
            ValidationPolicy::Inspect,
        )
        .unwrap();
    for binding in &mut config.bindings {
        binding.order = 1;
    }
    let manager = MessageSchemaManager::from_config(config);

    match manager.resolve_binding(1, "orders.created") {
        BindingResolution::Ambiguous(bindings) => {
            let ids: Vec<u64> = bindings.iter().map(|binding| binding.id).collect();
            assert!(ids.contains(&first));
            assert!(ids.contains(&second));
        }
        _ => panic!("expected ambiguous bindings"),
    }
}

#[test]
fn json_schema_validation_reports_invalid_payload() {
    let mut catalog = JsonSchemaCatalog {
        validators: HashMap::new(),
        schemas: HashMap::new(),
    };
    let schema = serde_json::json!({
        "type": "object",
        "required": ["id"],
        "properties": { "id": { "type": "string" } }
    });
    catalog.validators.insert(
        "order".to_string(),
        jsonschema::validator_for(&schema).unwrap(),
    );
    catalog.schemas.insert("order".to_string(), schema);

    assert!(
        catalog
            .validate("order", &serde_json::json!({ "id": "A1" }))
            .is_ok()
    );
    assert!(
        catalog
            .validate("order", &serde_json::json!({ "id": 1 }))
            .is_err()
    );
}

#[test]
fn json_schema_entry_name_uses_relative_directory_path() {
    let root = Path::new("D:/schemas/json");
    let direct = Path::new("D:/schemas/json/order-created.json");
    let nested = Path::new("D:/schemas/json/orders/created.schema.json");

    assert_eq!(schema_entry_name(root, direct), "order-created");
    assert_eq!(schema_entry_name(root, nested), "orders/created.schema");
}

#[test]
fn json_schema_template_uses_schema_hints() {
    let schema = serde_json::json!({
        "type": "object",
        "required": ["id", "items"],
        "properties": {
            "id": { "type": "string", "default": "ORD-1" },
            "items": {
                "type": "array",
                "items": { "type": "string", "examples": ["sku-1"] }
            },
            "paid": { "type": "boolean" }
        }
    });

    assert_eq!(
        json_schema_template(&schema),
        serde_json::json!({
            "id": "ORD-1",
            "items": ["sku-1"],
            "paid": false
        })
    );
}

#[test]
fn blocking_json_schema_binding_prevents_invalid_publish() {
    let mut config = MessageSchemaConfig::default();
    let source_id = config.add_source(
        "schemas".to_string(),
        SchemaSourceKind::JsonSchema,
        "unused".to_string(),
    );
    config
        .add_binding(
            "orders".to_string(),
            None,
            "orders.created".to_string(),
            source_id,
            SchemaSelector::JsonSchema {
                entry: "order".to_string(),
            },
            ValidationPolicy::Block,
        )
        .unwrap();
    let mut manager = MessageSchemaManager::from_config(config);
    let schema = serde_json::json!({
        "type": "object",
        "required": ["id"],
        "properties": { "id": { "type": "string" } }
    });
    let mut validators = HashMap::new();
    validators.insert(
        "order".to_string(),
        jsonschema::validator_for(&schema).unwrap(),
    );
    manager.json_sources.insert(
        source_id,
        JsonSchemaCatalog {
            validators,
            schemas: HashMap::from([("order".to_string(), schema)]),
        },
    );
    manager.statuses.insert(
        source_id,
        SchemaSourceStatus::loaded(vec!["order".to_string()]),
    );

    let invalid = manager.prepare_outgoing(1, "orders.created", r#"{"id":1}"#);
    assert!(!invalid.can_send);
    assert_eq!(
        invalid.status.as_ref().map(|status| status.level),
        Some(SchemaStatusLevel::Error)
    );

    let valid = manager.prepare_outgoing(1, "orders.created", r#"{"id":"A1"}"#);
    assert!(valid.can_send);
    assert_eq!(valid.payload, br#"{"id":"A1"}"#);
}

#[test]
fn protobuf_json_can_encode_and_decode_with_loaded_source() {
    let dir = unique_temp_dir("easy-nats-proto-test");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("order.proto"),
        r#"
                syntax = "proto3";
                package demo;
                message Order {
                    string id = 1;
                    int32 count = 2;
                }
            "#,
    )
    .unwrap();

    let mut manager = ProtoSchemaManager::default();
    manager.set_schema_dir(dir.clone());
    let bytes = manager
        .encode_json(r#"{"id":"A1","count":2}"#, "demo.Order")
        .unwrap();
    let json = manager.decode(&bytes, "demo.Order").unwrap();
    let template = manager.json_template("demo.Order").unwrap();

    assert!(json.contains(r#""id": "A1""#));
    assert!(json.contains(r#""count": 2"#));
    assert!(template.contains(r#""id": """#));
    assert!(template.contains(r#""count": 0"#));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn manual_proto_decode_uses_source_id_order_for_duplicate_types() {
    let first_dir = unique_temp_dir("easy-nats-proto-first");
    let second_dir = unique_temp_dir("easy-nats-proto-second");
    std::fs::create_dir_all(&first_dir).unwrap();
    std::fs::create_dir_all(&second_dir).unwrap();
    std::fs::write(
        first_dir.join("order.proto"),
        r#"
                syntax = "proto3";
                package demo;
                message Order {
                    string id = 1;
                }
            "#,
    )
    .unwrap();
    std::fs::write(
        second_dir.join("order.proto"),
        r#"
                syntax = "proto3";
                package demo;
                message Order {
                    string title = 1;
                }
            "#,
    )
    .unwrap();

    let mut first = ProtoSchemaManager::default();
    first.set_schema_dir(first_dir.clone());
    let bytes = first.encode_json(r#"{"id":"A1"}"#, "demo.Order").unwrap();
    let mut second = ProtoSchemaManager::default();
    second.set_schema_dir(second_dir.clone());

    let mut manager = MessageSchemaManager::default();
    manager.proto_sources.insert(20, second);
    manager.proto_sources.insert(10, first);

    let json = manager.decode_manual_proto(&bytes, "demo.Order").unwrap();
    assert!(json.contains(r#""id": "A1""#));
    assert!(!json.contains(r#""title""#));

    let _ = std::fs::remove_dir_all(first_dir);
    let _ = std::fs::remove_dir_all(second_dir);
}

#[test]
fn legacy_proto_dir_import_creates_unbound_source_once() {
    let mut config = MessageSchemaConfig::default();
    let first = config.import_legacy_proto_dir("C:/schemas");
    let second = config.import_legacy_proto_dir("C:/schemas");

    assert!(first.is_some());
    assert_eq!(second, None);
    assert_eq!(config.sources.len(), 1);
    assert!(config.bindings.is_empty());
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "{prefix}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    path
}
