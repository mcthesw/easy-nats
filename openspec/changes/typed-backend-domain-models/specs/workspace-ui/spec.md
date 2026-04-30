## ADDED Requirements

### Requirement: Resource state uses typed backend models
The workspace UI SHALL store backend-derived resource lists and tab state as typed domain models rather than dynamic JSON values.

#### Scenario: KV tab displays typed bucket status
- **WHEN** the user opens a KV bucket tab
- **THEN** the tab state stores typed bucket status and displays current key count separately from stored historical value count

#### Scenario: Object Store tab displays typed metadata
- **WHEN** the user opens an Object Store bucket tab
- **THEN** bucket and object metadata are read from typed fields rather than JSON key lookups

#### Scenario: Stream tab displays typed messages and consumers
- **WHEN** the user views stream messages or consumers
- **THEN** message and consumer UI code reads typed fields rather than `serde_json::Value` keys

### Requirement: Search workspace consumes typed source snapshots
The Search Workspace SHALL build searchable source snapshots from typed tab state instead of directly inspecting backend JSON payloads.

#### Scenario: Stream result navigation uses typed sequence
- **WHEN** a search result points to a stream message
- **THEN** the workspace locates the message through a typed sequence field rather than `msg["sequence"]`

#### Scenario: KV value search uses typed history items
- **WHEN** KV value search includes history data
- **THEN** searchable text comes from typed KV history items rather than dynamic JSON values

### Requirement: Large touched UI files stay bounded
Rust source files modified by this change SHALL remain at or below 700 lines by splitting domain state, action, or renderer code into focused modules when necessary.

#### Scenario: KV migration touches a 700-line file
- **WHEN** the KV typed migration modifies the existing KV tab renderer
- **THEN** the implementation splits enough responsibility into focused modules so the modified file does not exceed 700 lines
