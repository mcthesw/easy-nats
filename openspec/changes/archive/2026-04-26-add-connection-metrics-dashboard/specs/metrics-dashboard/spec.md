## ADDED Requirements

### Requirement: View connection metrics dashboard
The system SHALL provide a dockable metrics dashboard tab for any connected connection profile that has a configured metrics endpoint. The dashboard SHALL request monitoring data from the configured endpoint and display the monitored endpoint identity, current health, and latest summary metrics for the associated server.

#### Scenario: Open metrics dashboard successfully
- **WHEN** user opens Metrics for a connected profile with a reachable metrics endpoint
- **THEN** a metrics tab opens and displays the latest health state plus summary data loaded from the monitoring endpoint

#### Scenario: Metrics dashboard is connection-scoped
- **WHEN** user opens Metrics for two different connected profiles
- **THEN** each profile gets its own metrics dashboard tab scoped to its configured endpoint and latest samples

### Requirement: Render rolling time-series charts from recent samples
The system SHALL retain a short in-memory history of recent successful metrics samples for each open metrics dashboard tab and SHALL render time-series charts for core server traffic and JetStream usage from that rolling history.

#### Scenario: Repeated refreshes extend chart history
- **WHEN** the metrics dashboard receives multiple successful refreshes over time
- **THEN** the charts update to show the newly collected samples in chronological order

#### Scenario: Metrics history is ephemeral
- **WHEN** user closes a metrics dashboard tab and later reopens it
- **THEN** the dashboard starts a new in-memory sample history instead of restoring historical data from disk

### Requirement: Support manual refresh and auto-refresh while open
The system SHALL allow the user to manually refresh a metrics dashboard on demand and SHALL support optional periodic refresh while the tab remains open.

#### Scenario: Manual refresh
- **WHEN** user clicks Refresh in the metrics dashboard
- **THEN** the system requests a fresh monitoring sample from the configured endpoint and updates the dashboard with the latest results

#### Scenario: Auto-refresh
- **WHEN** auto-refresh is enabled for an open metrics dashboard tab
- **THEN** the system periodically collects new monitoring samples and updates the charts without reopening the tab

### Requirement: Communicate monitoring availability and partial failure clearly
The system SHALL distinguish between loading, unavailable, partial, and stale monitoring states. When one monitoring endpoint fails but others succeed, the dashboard SHALL continue showing the successful sections and SHALL identify the failed section instead of failing the entire dashboard silently.

#### Scenario: Monitoring endpoint unreachable
- **WHEN** the configured metrics endpoint cannot be reached during refresh
- **THEN** the dashboard shows an unavailable or stale-monitoring state with an explanatory error message

#### Scenario: Partial monitoring failure
- **WHEN** one or more monitoring sub-endpoints fail but at least one summary endpoint succeeds
- **THEN** the dashboard keeps showing the successful data and marks the failed section as unavailable
