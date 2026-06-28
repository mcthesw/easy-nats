pub fn build_header_map(headers: &[(String, String)]) -> Result<async_nats::HeaderMap, String> {
    let mut map = async_nats::HeaderMap::new();
    for (name, value) in headers {
        let header_name = name
            .parse::<async_nats::HeaderName>()
            .map_err(|e| format!("Invalid header name {name:?}: {e}"))?;
        let header_value = value
            .parse::<async_nats::HeaderValue>()
            .map_err(|e| format!("Invalid header value for {name:?}: {e}"))?;
        map.insert(header_name, header_value);
    }
    Ok(map)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_header_map_single() {
        let headers = vec![("X-Key".to_string(), "Value1".to_string())];
        let map = build_header_map(&headers).unwrap();
        assert_eq!(map.get("X-Key").unwrap().to_string(), "Value1");
    }

    #[test]
    fn test_build_header_map_rejects_invalid_name() {
        let headers = vec![("Bad Header".to_string(), "value".to_string())];
        let error = build_header_map(&headers).unwrap_err();

        assert!(error.contains("Invalid header name"));
    }

    #[test]
    fn test_build_header_map_rejects_invalid_value() {
        let headers = vec![("X-Key".to_string(), "bad\rvalue".to_string())];
        let error = build_header_map(&headers).unwrap_err();

        assert!(error.contains("Invalid header value"));
    }

    #[test]
    fn test_extract_headers_roundtrip() {
        let original = vec![
            ("X-Foo".to_string(), "bar".to_string()),
            ("X-Baz".to_string(), "qux".to_string()),
        ];
        let map = build_header_map(&original).unwrap();
        let extracted = extract_headers(&Some(map));
        assert!(extracted.iter().any(|(k, v)| k == "X-Foo" && v == "bar"));
        assert!(extracted.iter().any(|(k, v)| k == "X-Baz" && v == "qux"));
    }
}
