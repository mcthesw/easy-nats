## MODIFIED Requirements

### Requirement: Fixed sidebar with server resource tree
The system SHALL provide a fixed left `egui::SidePanel` containing a collapsible tree of all connection profiles and their resources (Pub/Sub, Streams, KV Buckets, Object Store Buckets, and Metrics for eligible connections). All icons in the sidebar header SHALL render correctly.

#### Scenario: Sidebar displays server tree
- **WHEN** the application is running with saved connection profiles
- **THEN** the left sidebar shows each server with its connection status and expandable resource categories

#### Scenario: Sidebar shows metrics entry for eligible connection
- **WHEN** a connection profile is connected and has a configured metrics monitoring endpoint
- **THEN** its resource tree includes a Metrics entry

#### Scenario: Sidebar hides metrics entry when unavailable
- **WHEN** a connection profile is disconnected or has no configured metrics monitoring endpoint
- **THEN** its resource tree does not show a Metrics entry

#### Scenario: Open resource as docked tab
- **WHEN** user clicks on a resource (e.g., stream `ORDERS` or Metrics) in the sidebar tree
- **THEN** if the tab is not already open, a new tab opens in the egui_dock DockArea for that resource

#### Scenario: Focus existing resource tab
- **WHEN** user clicks on a resource in the sidebar tree and a tab for that resource is already open
- **THEN** the existing tab is focused (brought to front and made active) instead of opening a duplicate or doing nothing

#### Scenario: Sidebar header icons render correctly
- **WHEN** the sidebar header is displayed
- **THEN** all icon buttons (⚙, 📋, ＋, ⏏, ●) render their intended glyphs without square-block fallback, using egui's default emoji fonts as fallback after custom fonts
