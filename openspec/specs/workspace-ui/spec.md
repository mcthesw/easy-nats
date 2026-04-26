## Purpose
Keep destructive dialogs consistent and remove dead legacy workspace UI behaviors from the spec.

## Requirements

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

### Requirement: egui_dock hybrid floating + docking workspace
The system SHALL use `egui_dock::DockArea` as the main content area, supporting both docked tabs (split/tabbed) and floating windows (undocked tabs rendered as `egui::Window`).

#### Scenario: Dock tabs side by side
- **WHEN** user drags a tab to the split zone of another tab
- **THEN** the two tabs are displayed side by side in a split layout

#### Scenario: Undock tab to floating window
- **WHEN** user drags a tab out of the dock area
- **THEN** the tab becomes a floating `egui::Window` that can be freely moved and resized

#### Scenario: Re-dock floating window
- **WHEN** user drags a floating window back into the dock area
- **THEN** the window docks as a tab in the target location

### Requirement: Tab title format with server origin
The system SHALL display tab titles in the format "ResourceName (ServerName)" to distinguish resources from different servers in the mixed workspace.

#### Scenario: Same resource name on different servers
- **WHEN** user opens stream "ORDERS" from server A and stream "ORDERS" from server B
- **THEN** the tabs are titled "ORDERS (Server-A)" and "ORDERS (Server-B)" respectively

### Requirement: Cross-server window mixing
The system SHALL allow tabs from different NATS servers to coexist in the same dock area. Users can arrange tabs from any server side by side.

#### Scenario: Cross-server layout
- **WHEN** user docks a stream tab from server A next to a KV tab from server B
- **THEN** both tabs are visible simultaneously in the split layout

### Requirement: Dark and light theme
The system SHALL provide a selectable application theme catalog consisting of egui's built-in `egui-dark` and `egui-light` themes plus all Catppuccin themes supported by `catppuccin-egui` (Latte, Frappé, Macchiato, and Mocha). The selected theme SHALL apply immediately, SHALL persist across launches, and SHALL be resolved at startup from the saved preference when present or from the OS dark/light preference when no explicit theme has been saved yet.

#### Scenario: Theme selector lists all supported themes
- **WHEN** the user opens appearance settings
- **THEN** the theme selector offers `egui-dark`, `egui-light`, Latte, Frappé, Macchiato, and Mocha as selectable themes

#### Scenario: Saved theme overrides system preference
- **WHEN** the user has previously saved a theme selection
- **THEN** the application starts with that exact theme even if the operating system preference differs

#### Scenario: Startup falls back to system preference without saved theme
- **WHEN** no explicit theme has been saved yet and the OS reports a dark or light preference
- **THEN** the application starts with `egui-dark` for dark mode or `egui-light` for light mode

#### Scenario: Legacy dark mode setting is migrated
- **WHEN** an existing settings file contains only the legacy `dark_mode` preference
- **THEN** the application maps that value to `egui-dark` or `egui-light` and preserves the user's previous appearance choice

#### Scenario: Theme selection applies immediately
- **WHEN** the user selects a different theme in settings
- **THEN** the UI switches to the selected theme in the same interaction without requiring restart or a second click

#### Scenario: Theme selection persists across launches
- **WHEN** the user selects Latte, Frappé, Macchiato, Mocha, `egui-dark`, or `egui-light` and later restarts the application
- **THEN** the application restores that same theme on the next launch

### Requirement: Confirmation dialogs for destructive actions
The system SHALL show a confirmation dialog before any destructive action (delete stream, purge messages, delete bucket, delete connection). Dialog windows SHALL use explicit Cancel/action buttons only and SHALL NOT use the egui title-bar X button (`.open()`).

#### Scenario: Delete with confirmation
- **WHEN** user initiates a delete action
- **THEN** a modal dialog appears asking for confirmation before proceeding

#### Scenario: Dialog has no title-bar X button
- **WHEN** a confirmation or editor dialog window is displayed
- **THEN** the window title bar does NOT have an X close button; the user closes the dialog via Cancel, Save, or other explicit buttons inside the window

#### Scenario: Consistent close pattern across all dialogs
- **WHEN** the user interacts with any dialog window (connection editor, stream create, consumer create, KV bucket create, object store dialog)
- **THEN** all dialogs use the same close mechanism (explicit buttons only, no `.open()`)

## REMOVED Requirements

### Requirement: Dead legacy KV key listing handler
**Reason**: The `list_kv_keys` operation handler in `kv_results.rs` was a dead compatibility path. The backend now exclusively sends `list_kv_keys_page` events.
**Migration**: No migration needed. The `list_kv_keys_page` handler fully replaces the old behavior.

### Requirement: Toast notifications for operations
The system SHALL display non-blocking toast notifications for operation results (success, error, info).

#### Scenario: Successful operation toast
- **WHEN** user creates a stream successfully
- **THEN** a success toast appears briefly showing "Stream created"

#### Scenario: Error toast on failure
- **WHEN** a stream creation fails due to server error
- **THEN** an error toast appears showing the error message

### Requirement: Centralized UI strings for future i18n
The system SHALL use a YAML-based i18n system for all user-facing text, supporting multiple languages with runtime switching. This replaces the previous `ui_strings.rs` constant-based approach.

#### Scenario: All strings via i18n
- **WHEN** a developer adds a new UI label
- **THEN** the string is added to the appropriate YAML translation file and referenced via the i18n lookup function

### Requirement: Resizable message list and preview split
The system SHALL allow users to drag-resize the vertical split between the message list and the message preview pane in Subscriber and Stream tabs. The split ratio SHALL be persisted per tab.

#### Scenario: Drag to resize split
- **WHEN** user drags the divider between message list and preview pane downward
- **THEN** the message list area grows and the preview area shrinks proportionally

#### Scenario: Split ratio persists
- **WHEN** user adjusts the split ratio and switches to another tab then back
- **THEN** the split ratio is preserved as previously set

### Requirement: Full-width message list
The system SHALL render the subscriber message list at the full available horizontal width of the tab, without unnecessary padding or constrained inner widths.

#### Scenario: Message list fills width
- **WHEN** user views the subscriber tab in a wide dock area
- **THEN** the message list columns expand to use all available horizontal space

### Requirement: Default branded welcome background
The system SHALL display a styled Welcome tab with application branding, logo, and quick-action links when no other tabs are open or on first launch. The welcome tab SHALL use egui-native widgets only.

#### Scenario: Welcome tab on first launch
- **WHEN** user launches the application for the first time
- **THEN** a Welcome tab is displayed with the app name, version, and quick actions (e.g., "New Connection")

#### Scenario: Welcome tab when all tabs closed
- **WHEN** user closes all tabs
- **THEN** the Welcome tab is shown automatically

### Requirement: Tab context menu
The system SHALL provide a right-click context menu on tab headers with actions: Close, Close Others, Close All, Close to the Right.

#### Scenario: Right-click context menu
- **WHEN** user right-clicks on a tab header
- **THEN** a context menu appears with Close, Close Others, Close All, and Close to the Right options

### Requirement: UX layout polish
The system SHALL maintain consistent spacing, alignment, and layout proportions across all views, strictly using egui's native styling without custom visual components. Scrollable content areas SHALL avoid unwanted horizontal scrollbars by constraining text widgets and other expanding content to the available width.

#### Scenario: Consistent spacing
- **WHEN** user navigates between different tab types (Publisher, Subscriber, Stream, KV)
- **THEN** margins, padding, and element spacing are visually consistent across all views

#### Scenario: No unwanted horizontal scrollbar in stream consumer view
- **WHEN** user expands a consumer card and views fetched messages in the stream detail tab
- **THEN** no unwanted horizontal scrollbar appears at the bottom of the view

#### Scenario: No unwanted horizontal scrollbar in bounded detail panels
- **WHEN** user views bounded text content inside stream, KV, or object store detail panels
- **THEN** the content stays within the available width instead of forcing horizontal overflow
### Requirement: Roadmap reflects search phases
The project roadmap SHALL distinguish the scoped in-tab search delivered by this change from the future global in-memory full-text search tab. The roadmap SHALL avoid listing already shipped distribution channels as pending work.

#### Scenario: Scoped search is documented separately from global search
- **WHEN** a reader checks the roadmap after this change
- **THEN** they can see that v1 focuses on scoped search of fetched tab content while a later roadmap item covers a global in-memory full-text search tab

#### Scenario: Completed distribution work is not shown as pending
- **WHEN** a reader checks distribution roadmap status
- **THEN** channels already described as available in the README are not duplicated as pending setup items

### Requirement: Global Search Workspace entry point
The system SHALL provide a discoverable workspace-level action that opens or focuses the Search Workspace tab without requiring the user to select a specific connection or resource first.

#### Scenario: Open search workspace
- **WHEN** user activates the workspace search action
- **THEN** a Search Workspace tab opens in the egui_dock DockArea

#### Scenario: Focus existing search workspace
- **WHEN** user activates the workspace search action while a Search Workspace tab is already open
- **THEN** the existing Search Workspace tab is focused instead of opening a duplicate

### Requirement: Search Workspace layout
The Search Workspace tab SHALL use a compact operational layout with a query row, selected source controls, result list, and preview/actions area. The layout SHALL remain readable in both docked and floating window modes and SHALL avoid long instructional text in normal loaded states.

#### Scenario: Workspace shows main controls together
- **WHEN** user opens the Search Workspace with selected sources
- **THEN** the query input, field/source controls, selected source coverage, results, and preview are visible without leaving the tab

#### Scenario: Workspace handles narrow dock areas
- **WHEN** the Search Workspace is docked in a narrow area
- **THEN** controls wrap or collapse without overlapping text or making result rows unreadable

#### Scenario: Results are grouped by source
- **WHEN** selected sources produce search matches
- **THEN** the result list shows each source label once with coverage metadata, followed by matching entries from that source

#### Scenario: No noisy healthy-state copy
- **WHEN** selected sources are searchable and the query has results
- **THEN** the workspace avoids extra explanatory status text beyond source coverage, result counts, and selected-result metadata
