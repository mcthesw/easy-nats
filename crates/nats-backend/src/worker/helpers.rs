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
