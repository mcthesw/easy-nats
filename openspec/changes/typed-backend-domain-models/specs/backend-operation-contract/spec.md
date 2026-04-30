## ADDED Requirements

### Requirement: Typed domain operation results
Backend operation results for internal resource domains SHALL be represented by Rust structs and `BackendEvent` variants instead of `serde_json::Value` payloads.

#### Scenario: KV bucket status uses typed data
- **WHEN** the backend reports KV bucket status
- **THEN** the event exposes typed fields for bucket name, stored historical value count, history depth, storage, byte usage, and limits without requiring the app to read JSON keys

#### Scenario: Migrated domains remove JSON fallbacks
- **WHEN** a domain slice has been migrated to typed events
- **THEN** that domain no longer accepts fallback JSON field names for the same data

### Requirement: Explicit dynamic JSON boundaries
The system SHALL allow `serde_json::Value` only for explicitly dynamic data boundaries: user payload formatting, JSON Schema documents, protobuf JSON templates, and parser internals for external JSON APIs.

#### Scenario: JSON Schema remains dynamic
- **WHEN** the app loads and validates a user-provided JSON Schema
- **THEN** schema parsing may continue to use `serde_json::Value`

#### Scenario: Domain metadata is not dynamic JSON
- **WHEN** the app stores backend-derived stream, KV, object store, consumer, or server metadata
- **THEN** the stored value is a typed domain model rather than `serde_json::Value`

### Requirement: Typed command configuration inputs
Create and update commands for backend-managed resources SHALL use typed config structs instead of generic JSON config payloads.

#### Scenario: KV bucket update uses typed config
- **WHEN** the app sends a KV bucket create or update request
- **THEN** the backend receives a typed KV bucket config model and performs conversion to async-nats config inside the backend layer

#### Scenario: Consumer edit does not keep original JSON config
- **WHEN** the user edits a consumer
- **THEN** the editor retains a typed editable config snapshot rather than the raw consumer JSON returned by the backend

### Requirement: Typed error context
Backend errors that need context for app cleanup SHALL carry typed error context instead of `Option<serde_json::Value>`.

#### Scenario: KV entry request failure identifies target
- **WHEN** a KV entry fetch fails
- **THEN** the error event includes typed bucket and key context so the app clears only matching loading or scan state
