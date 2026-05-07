use crate::tabs::TabKind;

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
        (
            TabKind::ServerInfo {
                connection_id: a1, ..
            },
            TabKind::ServerInfo {
                connection_id: a2, ..
            },
        ) => a1 == a2,
        (
            TabKind::Metrics {
                connection_id: a1, ..
            },
            TabKind::Metrics {
                connection_id: a2, ..
            },
        ) => a1 == a2,
        (
            TabKind::Clients {
                connection_id: a1, ..
            },
            TabKind::Clients {
                connection_id: a2, ..
            },
        ) => a1 == a2,
        (TabKind::SearchWorkspace { .. }, TabKind::SearchWorkspace { .. }) => true,
        (TabKind::MessageSchemas { .. }, TabKind::MessageSchemas { .. }) => true,
        (TabKind::Settings, TabKind::Settings) => true,
        (TabKind::LogViewer, TabKind::LogViewer) => true,
        _ => false,
    }
}
