use std::time::SystemTime;

use reqwest::Url;

use crate::monitoring::{
    ClientConnectionState, ClientStatusQuery, ClientStatusSort, ClientStatusSubscription,
};

use super::{
    ConnzResponse, build_client_status_detail_url, build_client_status_page_url,
    normalize_client_status_detail, normalize_client_status_page,
};

#[test]
fn default_client_status_page_query_is_open_and_bounded() {
    let query = ClientStatusQuery::default();
    let url =
        build_client_status_page_url(Url::parse("http://localhost:8222").unwrap(), &query).unwrap();

    assert_eq!(query.state, ClientConnectionState::Open);
    assert_eq!(query.sort, ClientStatusSort::Cid);
    assert_eq!(query.page_size, 100);
    assert_eq!(query.offset, 0);
    assert_eq!(query.client_id, None);
    assert!(query.include_auth);
    assert!(!query.include_subscriptions);
    assert_eq!(
        url.as_str(),
        "http://localhost:8222/connz?limit=100&offset=0&sort=cid&state=open&auth=1"
    );
}

#[test]
fn client_status_page_query_uses_filter_sort_limit_and_offset() {
    let query = ClientStatusQuery {
        state: ClientConnectionState::Closed,
        sort: ClientStatusSort::PendingBytes,
        page_size: 250,
        offset: 500,
        client_id: None,
        include_subscriptions: false,
        include_auth: true,
    };
    let url = build_client_status_page_url(
        Url::parse("http://localhost:8222/nested/path").unwrap(),
        &query,
    )
    .unwrap();

    assert_eq!(
        url.as_str(),
        "http://localhost:8222/connz?limit=250&offset=500&sort=pending&state=closed&auth=1"
    );
}

#[test]
fn selected_client_detail_query_requests_subscriptions_for_only_that_client() {
    let query = ClientStatusQuery::detail(42);
    let url = build_client_status_detail_url(Url::parse("http://localhost:8222").unwrap(), &query)
        .unwrap();

    assert_eq!(query.client_id, Some(42));
    assert!(query.include_subscriptions);
    assert_eq!(
        url.as_str(),
        "http://localhost:8222/connz?cid=42&state=open&subs=1&auth=1"
    );
}

#[test]
fn selected_client_detail_query_preserves_state_filter() {
    let query = ClientStatusQuery {
        state: ClientConnectionState::Closed,
        ..ClientStatusQuery::detail(42)
    };
    let url = build_client_status_detail_url(Url::parse("http://localhost:8222").unwrap(), &query)
        .unwrap();

    assert_eq!(
        url.as_str(),
        "http://localhost:8222/connz?cid=42&state=closed&subs=1&auth=1"
    );
}

#[test]
fn connz_open_page_is_normalized_into_client_rows() {
    let raw: ConnzResponse = serde_json::from_str(
        r#"{
            "num_connections": 2,
            "total": 12,
            "offset": 100,
            "limit": 2,
            "connections": [
                {
                    "cid": 41,
                    "name": "orders-api",
                    "account": "APP",
                    "user": "alice",
                    "ip": "127.0.0.1",
                    "port": 52310,
                    "uptime": "2m",
                    "idle": "1s",
                    "last_activity": "2026-05-07T10:00:00Z",
                    "rtt": "500µs",
                    "subscriptions": 5,
                    "pending_bytes": 7,
                    "in_msgs": 11,
                    "out_msgs": 13,
                    "in_bytes": 17,
                    "out_bytes": 19,
                    "lang": "rust",
                    "version": "1.0.0"
                },
                {
                    "cid": 42
                }
            ]
        }"#,
    )
    .unwrap();

    let page = normalize_client_status_page(
        "http://localhost:8222".to_string(),
        SystemTime::UNIX_EPOCH,
        ClientStatusQuery::default(),
        raw,
    );

    assert_eq!(page.total, 12);
    assert_eq!(page.offset, 100);
    assert_eq!(page.limit, 2);
    assert_eq!(page.clients.len(), 2);
    let first = &page.clients[0];
    assert_eq!(first.client_id, 41);
    assert_eq!(first.state, ClientConnectionState::Open);
    assert_eq!(first.name.as_deref(), Some("orders-api"));
    assert_eq!(first.account.as_deref(), Some("APP"));
    assert_eq!(first.user.as_deref(), Some("alice"));
    assert_eq!(first.remote_address(), Some("127.0.0.1:52310".to_string()));
    assert_eq!(first.subscriptions, Some(5));
    assert_eq!(first.pending_bytes, Some(7));
    assert_eq!(first.in_msgs, Some(11));
    assert_eq!(first.out_msgs, Some(13));
    assert_eq!(first.in_bytes, Some(17));
    assert_eq!(first.out_bytes, Some(19));
    assert_eq!(first.language.as_deref(), Some("rust"));
    assert_eq!(first.version.as_deref(), Some("1.0.0"));
    assert_eq!(page.clients[1].client_id, 42);
    assert_eq!(page.clients[1].subscriptions, None);
}

#[test]
fn connz_closed_page_preserves_close_reason() {
    let raw: ConnzResponse = serde_json::from_str(
        r#"{
            "num_connections": 1,
            "total": 1,
            "connections": [
                {
                    "cid": 7,
                    "stop": "2026-05-07T10:05:00Z",
                    "reason": "Client Closed"
                }
            ]
        }"#,
    )
    .unwrap();

    let page = normalize_client_status_page(
        "http://localhost:8222".to_string(),
        SystemTime::UNIX_EPOCH,
        ClientStatusQuery {
            state: ClientConnectionState::Closed,
            ..Default::default()
        },
        raw,
    );

    assert_eq!(page.clients[0].state, ClientConnectionState::Closed);
    assert_eq!(
        page.clients[0].closed_reason.as_deref(),
        Some("Client Closed")
    );
    assert_eq!(
        page.clients[0].closed_at.as_deref(),
        Some("2026-05-07T10:05:00Z")
    );
}

#[test]
fn selected_client_detail_keeps_subscription_subjects() {
    let raw: ConnzResponse = serde_json::from_str(
        r#"{
            "num_connections": 1,
            "total": 1,
            "connections": [
                {
                    "cid": 9,
                    "subscriptions": 2,
                    "subscriptions_list": ["orders.*", "events.>"]
                }
            ]
        }"#,
    )
    .unwrap();

    let detail = normalize_client_status_detail(
        "http://localhost:8222".to_string(),
        SystemTime::UNIX_EPOCH,
        ClientStatusQuery::detail(9),
        raw,
    )
    .unwrap();

    assert_eq!(detail.client.client_id, 9);
    assert_eq!(
        detail.client.subscription_details,
        vec![
            ClientStatusSubscription {
                subject: "orders.*".to_string()
            },
            ClientStatusSubscription {
                subject: "events.>".to_string()
            }
        ]
    );
}
