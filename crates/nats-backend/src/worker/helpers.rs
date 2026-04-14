use base64::Engine;

pub fn build_header_map(headers: &[(String, String)]) -> async_nats::HeaderMap {
    let mut map = async_nats::HeaderMap::new();
    for (k, v) in headers {
        map.insert(k.as_str(), v.as_str());
    }
    map
}

pub fn extract_headers(headers: &Option<async_nats::HeaderMap>) -> Vec<(String, String)> {
    let Some(map) = headers else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for (name, values) in map.iter() {
        for value in values.iter() {
            result.push((name.to_string(), value.to_string()));
        }
    }
    result
}

pub(crate) fn raw_message_to_json(
    msg: &async_nats::jetstream::message::StreamMessage,
) -> serde_json::Value {
    let payload_b64 = base64::engine::general_purpose::STANDARD.encode(&msg.payload);
    let mut headers = Vec::new();
    for (name, values) in msg.headers.iter() {
        for value in values.iter() {
            headers.push(serde_json::json!([name.to_string(), value.to_string()]));
        }
    }
    serde_json::json!({
        "sequence": msg.sequence,
        "subject": msg.subject.to_string(),
        "payload_base64": payload_b64,
        "headers": headers,
        "time": msg.time.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
    })
}

pub(crate) fn stream_info_to_json(info: &async_nats::jetstream::stream::Info) -> serde_json::Value {
    let config_json = serde_json::to_value(&info.config).unwrap_or_default();
    serde_json::json!({
        "config": config_json,
        "state": {
            "messages": info.state.messages,
            "bytes": info.state.bytes,
            "first_sequence": info.state.first_sequence,
            "last_sequence": info.state.last_sequence,
            "consumer_count": info.state.consumer_count,
        },
    })
}

pub(crate) fn consumer_info_to_json(
    info: &async_nats::jetstream::consumer::Info,
) -> serde_json::Value {
    let config_json = serde_json::to_value(&info.config).unwrap_or_default();
    serde_json::json!({
        "name": info.name,
        "stream_name": info.stream_name,
        "config": config_json,
        "num_pending": info.num_pending,
        "num_ack_pending": info.num_ack_pending,
        "num_waiting": info.num_waiting,
        "num_redelivered": info.num_redelivered,
        "push_bound": info.push_bound,
    })
}

pub(crate) fn kv_status_to_json(
    status: &async_nats::jetstream::kv::bucket::Status,
) -> serde_json::Value {
    serde_json::json!({
        "bucket": status.bucket(),
        "values": status.values(),
        "history": status.history(),
        "max_age_secs": status.max_age().as_secs(),
        "description": status.info.config.description,
        "storage": format!("{:?}", status.info.config.storage),
        "bytes": status.info.state.bytes,
        "max_bytes": status.info.config.max_bytes,
    })
}

pub(crate) fn kv_entry_to_json(entry: &async_nats::jetstream::kv::Entry) -> serde_json::Value {
    serde_json::json!({
        "bucket": entry.bucket,
        "key": entry.key,
        "value_base64": base64::engine::general_purpose::STANDARD.encode(&entry.value),
        "revision": entry.revision,
        "delta": entry.delta,
        "created": entry.created.to_string(),
        "operation": format!("{:?}", entry.operation),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_header_map_single() {
        let headers = vec![("X-Key".to_string(), "Value1".to_string())];
        let map = build_header_map(&headers);
        assert_eq!(map.get("X-Key").unwrap().to_string(), "Value1");
    }

    #[test]
    fn test_extract_headers_roundtrip() {
        let original = vec![
            ("X-Foo".to_string(), "bar".to_string()),
            ("X-Baz".to_string(), "qux".to_string()),
        ];
        let map = build_header_map(&original);
        let extracted = extract_headers(&Some(map));
        assert!(extracted.iter().any(|(k, v)| k == "X-Foo" && v == "bar"));
        assert!(extracted.iter().any(|(k, v)| k == "X-Baz" && v == "qux"));
    }
}
