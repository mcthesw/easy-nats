use std::sync::{Arc, Mutex};

use nats_backend::{AuthMethod, ConnectionConfig, MonitoringConfig};

use super::EasyNatsApp;
use crate::log_layer::LogBuffer;
use crate::settings::{AppSettings, PubSubTabMode};
use crate::tabs::TabKind;
use crate::theme::ThemeId;

fn test_app(mode: PubSubTabMode) -> EasyNatsApp {
    EasyNatsApp::new(
        AppSettings {
            pubsub_tab_mode: mode,
            ..Default::default()
        },
        ThemeId::EguiDark,
        Arc::new(Mutex::new(LogBuffer::default())),
    )
}

fn push_metrics_connection(app: &mut EasyNatsApp, connection_id: u64) {
    app.config.connections.push(ConnectionConfig {
        id: connection_id,
        name: "local".to_string(),
        urls: vec!["nats://localhost:4222".to_string()],
        auth: AuthMethod::None,
        tls_enabled: false,
        tls_first: false,
        monitoring: Some(MonitoringConfig {
            endpoint: "http://localhost:8222".to_string(),
        }),
    });
}

fn count_publisher_tabs(app: &EasyNatsApp, connection_id: u64) -> usize {
    app.dock_state
        .iter_all_tabs()
        .filter(|(_, tab)| {
            matches!(
                tab,
                TabKind::Publisher {
                    connection_id: existing_id,
                    ..
                } if *existing_id == connection_id
            )
        })
        .count()
}

fn count_subscriber_tabs(app: &EasyNatsApp, connection_id: u64) -> usize {
    app.dock_state
        .iter_all_tabs()
        .filter(|(_, tab)| {
            matches!(
                tab,
                TabKind::Subscriber {
                    connection_id: existing_id,
                    ..
                } if *existing_id == connection_id
            )
        })
        .count()
}

fn count_metrics_tabs(app: &EasyNatsApp, connection_id: u64) -> usize {
    app.dock_state
        .iter_all_tabs()
        .filter(|(_, tab)| {
            matches!(
                tab,
                TabKind::Metrics {
                    connection_id: existing_id,
                    ..
                } if *existing_id == connection_id
            )
        })
        .count()
}

#[test]
fn publisher_reuses_existing_tab_when_configured() {
    let mut app = test_app(PubSubTabMode::ReuseExisting);

    app.open_or_focus_publisher_tab(7);
    app.open_or_focus_publisher_tab(7);

    assert_eq!(count_publisher_tabs(&app, 7), 1);
}

#[test]
fn subscriber_opens_new_tabs_by_default() {
    let mut app = test_app(PubSubTabMode::NewTab);

    app.open_or_focus_subscriber_tab(7);
    app.open_or_focus_subscriber_tab(7);

    assert_eq!(count_subscriber_tabs(&app, 7), 2);
}

#[test]
fn metrics_tab_focuses_existing_tab_for_same_connection() {
    let mut app = test_app(PubSubTabMode::NewTab);
    push_metrics_connection(&mut app, 7);

    app.open_or_focus_metrics_tab(7);
    app.open_or_focus_metrics_tab(7);

    assert_eq!(count_metrics_tabs(&app, 7), 1);
}
