use base64::Engine;

use crate::tabs::TabKind;

pub(crate) fn decode_kv_value(entry: &serde_json::Value) -> String {
    entry["value_base64"]
        .as_str()
        .and_then(|value| base64::engine::general_purpose::STANDARD.decode(value).ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
        .unwrap_or_default()
}

pub(crate) fn same_tab(a: &TabKind, b: &TabKind) -> bool {
    match (a, b) {
        (TabKind::Welcome, TabKind::Welcome) => true,
        // Publisher and Subscriber tabs are always unique instances
        (TabKind::Publisher { .. }, TabKind::Publisher { .. }) => false,
        (TabKind::Subscriber { .. }, TabKind::Subscriber { .. }) => false,
        (
            TabKind::Stream {
                connection_id: a1,
                stream_name: s1,
                ..
            },
            TabKind::Stream {
                connection_id: a2,
                stream_name: s2,
                ..
            },
        ) => a1 == a2 && s1 == s2,
        (
            TabKind::KvBucket {
                connection_id: a1,
                bucket_name: b1,
                ..
            },
            TabKind::KvBucket {
                connection_id: a2,
                bucket_name: b2,
                ..
            },
        ) => a1 == a2 && b1 == b2,
        (
            TabKind::ObjectStoreBucket {
                connection_id: a1,
                bucket_name: b1,
                ..
            },
            TabKind::ObjectStoreBucket {
                connection_id: a2,
                bucket_name: b2,
                ..
            },
        ) => a1 == a2 && b1 == b2,
        (TabKind::Settings, TabKind::Settings) => true,
        (TabKind::LogViewer, TabKind::LogViewer) => true,
        _ => false,
    }
}
