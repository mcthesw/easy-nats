## Context

easy-nats already has a clear connection model, a sidebar resource tree, dockable tabs, and several tab types that use on-demand backend fetches plus optional auto-refresh. It also already exposes one-shot server information and JetStream account details, and the sidebar i18n catalog even reserves a Metrics section label, but there is no continuous monitoring workflow yet.

This change cuts across persisted connection configuration, backend commands, async HTTP fetching, app state, sidebar navigation, and a new tab renderer. The design also needs to respect existing project constraints: native desktop only, egui-native styling, minimal unnecessary background work, and reasonable dependency discipline after recent binary-size optimization work.

NATS provides native monitoring JSON endpoints on a dedicated HTTP/HTTPS port, including `/healthz`, `/varz`, `/connz`, and `/jsz`. Those endpoints are directly useful for a dashboard view, but they are optional server features and are not tied to the main NATS client URL. NATS also exposes similar data through `$SYS` system services, but that path requires system-account privileges and is not guaranteed to work for ordinary application credentials.

## Goals / Non-Goals

**Goals:**
- Let users optionally associate a monitoring endpoint with each saved NATS connection.
- Provide a connection-scoped metrics dashboard tab with a short rolling history and charted trends.
- Reuse existing easy-nats interaction patterns: sidebar entry, dock tab, explicit refresh, auto-refresh cadence, and native egui widgets.
- Surface core operational signals quickly: health, connections, subscriptions, traffic rates, memory, CPU, and JetStream usage.
- Fail clearly when monitoring is not configured, not enabled on the server, or temporarily unreachable.

**Non-Goals:**
- Building a full Prometheus or Grafana replacement with arbitrary queries, alerting, dashboards, or long-term storage.
- Persisting historical metrics across restarts.
- Auto-discovering or mutating server monitoring configuration.
- Aggregating cluster-wide metrics across multiple monitoring endpoints in the first version.
- Supporting authenticated reverse-proxy monitoring endpoints, custom headers, or Prometheus exposition parsing in the first version.

## Decisions

### 1. Use an optional per-connection monitoring endpoint as the primary data source

**Choice**: Extend `ConnectionConfig` with an optional metrics/monitoring configuration that stores a base HTTP/HTTPS URL for NATS monitoring.

**Rationale**: This matches the user's workflow request, reflects how NATS monitoring is actually deployed, and avoids assuming that `nats://host:4222` implies `http://host:8222`. It also works whether the monitoring endpoint is local, proxied, or disabled.

**Alternatives considered**:
- *Infer `:8222` automatically from the NATS URL*: convenient but frequently wrong in production.
- *Use `$SYS.REQ.SERVER.*` system services over the existing NATS connection*: avoids HTTP, but requires system-account permissions and would fail for many ordinary users.
- *Use Prometheus exporter data first*: stronger observability story, but much heavier setup and not aligned with “optional endpoint on connection creation”.

### 2. Poll only while a metrics tab is open, and keep history tab-scoped

**Choice**: Metrics collection is demand-driven. Opening the metrics tab triggers fetches, and auto-refresh continues only while the tab remains open. History lives in the metrics tab state as a capped in-memory ring buffer.

**Rationale**: This follows existing easy-nats patterns, keeps background work proportional to user intent, and avoids a global metrics subsystem that runs for every connected server whether anyone is looking at it or not.

**Alternatives considered**:
- *Always-on per-connection polling*: better for instant-open dashboards, but adds background network traffic, more state management, and stale-data questions for hidden tabs.
- *App-global metrics cache keyed by connection*: useful if multiple views need the same history, but unnecessary for a single metrics tab per connection in v1.

### 3. Store raw snapshots and derive chart series from deltas

**Choice**: Each successful refresh stores a timestamped raw snapshot containing health status plus the normalized fields needed from `/varz`, `/connz`, and `/jsz`. The UI derives rate series such as messages/sec and bytes/sec from adjacent cumulative counters.

**Rationale**: Raw snapshots preserve fidelity, make it easier to change chart formulas later, and handle irregular polling intervals more safely than storing only precomputed rates.

**Alternatives considered**:
- *Store only already-computed rates*: simpler rendering path, but less flexible and more error-prone when polls are delayed.
- *Persist history to disk*: out of scope for v1 and adds migration/storage complexity.

### 4. Use `egui_plot` for primary charts and `egui::Painter` only for small supporting visuals

**Choice**: Primary time-series charts use `egui_plot` with legends, hover labels, zoom/pan, and custom axis formatting. Supporting chrome such as status cards, compact sparklines, or separators can be built with normal egui layouts and targeted custom painting.

**Rationale**: `egui_plot` already provides the interaction model expected for dashboard charts inside an egui app, while `Painter` gives enough flexibility to make the surrounding layout feel more like an observability dashboard without abandoning egui-native styling.

**Alternatives considered**:
- *Custom-paint every chart*: maximum visual control, but too much implementation cost for basic plotting behaviors that `egui_plot` already solves.
- *Render charts via a bitmap-first library such as `plotters`*: workable, but a worse fit for immediate-mode interaction and crosshair/hover behavior.

### 5. Start with a focused metrics set and a single-node dashboard model

**Choice**: The first dashboard focuses on:
- endpoint health from `/healthz`
- server runtime and traffic counters from `/varz`
- connection counts from `/connz`
- JetStream usage from `/jsz`

The dashboard is explicitly node-scoped to the configured endpoint, even if the NATS deployment is clustered.

**Rationale**: These endpoints provide the highest-value operational signals with a manageable data model and without paging-heavy or low-signal views such as `/subsz` or `/routez`.

**Alternatives considered**:
- *Include every monitoring endpoint*: too much surface area and too many low-value charts for a first release.
- *Build cluster aggregation immediately*: attractive, but requires discovery and reconciliation rules that are outside the requested scope.

### 6. Model metrics config as a nested optional config, not a loose free-floating field

**Choice**: Add an optional nested metrics config under each connection, even if v1 only needs a single endpoint string.

**Rationale**: This keeps connection concerns grouped and gives the config room to grow later for features such as custom poll intervals, proxy/auth settings, or endpoint labels without another schema reshuffle.

**Alternatives considered**:
- *Flat `metrics_endpoint: Option<String>` field*: smaller change now, but more likely to force migration churn later.

### 7. Use a minimal async HTTP client inside the backend runtime

**Choice**: Fetch monitoring JSON in `nats-backend` using a feature-minimized async HTTP client configuration and return normalized metrics snapshots to the UI.

**Rationale**: The backend already owns async I/O and command/result transport. Keeping HTTP there avoids pushing networking into the egui render loop and keeps all remote data access behind one boundary.

**Alternatives considered**:
- *Fetch directly from the app crate*: simpler in the short term, but weakens the current UI/backend split.
- *Use blocking HTTP*: easier API surface, but mismatched with the existing Tokio worker design.

### 8. Expose distinct empty, partial, and stale states

**Choice**: The metrics tab distinguishes:
- no metrics configured
- loading first sample
- monitoring endpoint unreachable
- endpoint responded but some sub-endpoints failed
- data loaded but stale because recent refreshes failed

**Rationale**: Monitoring failures are normal operational conditions. The UI needs to explain whether the NATS connection itself is bad, the monitoring port is disabled, or only one part of the dashboard is currently unavailable.

## Risks / Trade-offs

- **[Monitoring endpoint is unauthenticated by default]** -> Document the expectation that users point to internal-only endpoints; do not auto-discover or encourage unsafe exposure.
- **[Extra dependencies can grow binary size]** -> Use feature-minimized plotting/HTTP dependencies and keep the first dashboard narrow.
- **[Polling and charts can become noisy on large servers]** -> Cap retained samples, poll only while the tab is open, and keep v1 charts focused on summary-level metrics.
- **[Configured endpoint may not correspond to the connected server node]** -> Treat the dashboard as explicitly endpoint-scoped and display the monitored host/URL in the tab.
- **[Cumulative counters are easy to misread]** -> Derive rates using timestamp deltas and label units clearly.
- **[Partial endpoint failures can confuse users]** -> Show per-section error copy and preserve the last good sample with a stale indicator instead of blanking the whole dashboard.

## Migration Plan

- Add the optional metrics config to persisted connection profiles with serde defaults so existing configs load unchanged.
- Extend the connection editor to create, edit, and clear the optional metrics endpoint without affecting normal NATS connectivity.
- Add backend commands/results and the new metrics tab behind the new sidebar entry.
- Keep metrics history ephemeral; no disk migration is required beyond the new optional config field.
- Rollback remains straightforward because older builds will ignore unknown persisted fields in connection JSON and the new tab state is not persisted.

## Open Questions

- Should the connection editor offer a one-click suggestion derived from the NATS host (for example `localhost:8222`) while still keeping the field optional and user-controlled?
- Do we want v1 to support HTTPS endpoints with custom certificates immediately, or defer non-standard TLS handling until a later change?
