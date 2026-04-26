## 1. Connection configuration

- [x] 1.1 Add an optional persisted metrics configuration to `ConnectionConfig` with backward-compatible serde defaults
- [x] 1.2 Extend the connection editor state and window UI to create, edit, and clear the optional metrics endpoint
- [x] 1.3 Update connection save/load paths so metrics configuration survives profile edits and restarts

## 2. Backend monitoring pipeline

- [x] 2.1 Add the plotting and async HTTP dependencies needed for metrics fetching and rendering
- [x] 2.2 Introduce backend commands, result events, and normalized metrics snapshot types for monitoring data
- [x] 2.3 Implement async fetches for `/healthz`, `/varz`, `/connz`, and `/jsz` with per-endpoint error reporting

## 3. Metrics tab wiring

- [x] 3.1 Add a `Metrics` tab kind, tab state, and open-or-focus behavior keyed by connection identity
- [x] 3.2 Extend the sidebar to show a Metrics entry only for connected profiles with a configured metrics endpoint
- [x] 3.3 Wire backend metrics results into app event handling and tab state updates, including rolling in-memory sample history

## 4. Dashboard UI

- [x] 4.1 Build the metrics dashboard layout with monitored-endpoint identity, health badge, and summary stat cards
- [x] 4.2 Render time-series charts for traffic and JetStream usage using `egui_plot`
- [x] 4.3 Add manual refresh, auto-refresh controls, and empty/unavailable/partial/stale monitoring states

## 5. Strings and verification

- [x] 5.1 Add i18n strings for metrics configuration, sidebar labels, dashboard headings, and monitoring-state messages
- [x] 5.2 Add focused tests for config persistence, metrics tab identity/open-focus behavior, and snapshot/history handling
- [x] 5.3 Run `cargo fmt`, `cargo test -p easy-nats`, and `cargo clippy -p easy-nats --all-targets -- -D warnings`
