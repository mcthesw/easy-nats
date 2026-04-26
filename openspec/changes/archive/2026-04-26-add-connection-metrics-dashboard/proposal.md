## Why

easy-nats currently exposes server information and JetStream account usage as one-shot detail views, but it does not give operators a live sense of server load, connection churn, or JetStream pressure over time. NATS already exposes monitoring data through dedicated JSON endpoints, so adding an optional connection-scoped metrics dashboard now can turn the app from a resource browser into a lightweight operational console without forcing users to stand up Prometheus or Grafana first.

## What Changes

- Add an optional metrics endpoint field to connection profiles so users can associate a NATS monitoring HTTP/HTTPS address with a saved connection.
- Add a connection-level Metrics entry in the sidebar for connected profiles that have metrics configured.
- Introduce a dockable metrics dashboard tab that polls NATS monitoring endpoints and keeps a short in-memory history for charting.
- Display an overview layout with health/status badges, key stat cards, and time-series charts for core server throughput and JetStream usage.
- Start with NATS-native monitoring endpoints (`/healthz`, `/varz`, `/connz`, `/jsz`) rather than Prometheus scraping or Grafana-style query composition.
- Provide explicit empty, loading, unavailable, and misconfigured states so the dashboard fails clearly when monitoring is disabled or unreachable.
- Keep the visuals within egui-native styling while using richer panel composition and charts to achieve a more dashboard-like workflow.

## Capabilities

### New Capabilities
- `metrics-dashboard`: connection-scoped metrics polling, short-term history retention, and chart-based monitoring views for NATS servers.

### Modified Capabilities
- `connection-management`: connection profiles can optionally persist a monitoring endpoint alongside the main NATS server URL and credentials.
- `workspace-ui`: the sidebar resource tree can expose a Metrics entry for eligible connections and open a dedicated metrics tab.

## Impact

- `crates/nats-backend`: add monitoring fetch commands, HTTP polling, response parsing, and metrics-oriented result types.
- `crates/app/src/app`: extend connection editor flow, sidebar actions, app model state, event handling, and metrics tab open/focus behavior.
- `crates/app/src/tabs`: add a metrics dashboard tab state and UI renderer.
- `assets/i18n`: add labels, help text, status messages, and dashboard copy for metrics configuration and visualization.
- `crates/nats-backend/src/connection.rs` and persisted config handling: add optional metrics endpoint fields with backwards-compatible defaults.
- New dependencies are likely needed for plotting and HTTP access, most notably `egui_plot` and a minimal async HTTP client configuration.
