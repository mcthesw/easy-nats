## MODIFIED Requirements

### Requirement: Create new connection profile
The system SHALL allow users to create a NATS connection profile by specifying: display name, server URL(s), optional authentication credentials, and an optional metrics monitoring endpoint.

#### Scenario: Minimal connection creation
- **WHEN** user provides a display name and a server URL (e.g., `nats://localhost:4222`)
- **THEN** a connection profile is saved and appears in the connection list without requiring a metrics endpoint

#### Scenario: Connection creation with metrics endpoint
- **WHEN** user provides a display name, a server URL, and a metrics monitoring endpoint
- **THEN** the connection profile is saved with both the NATS connection settings and the optional metrics endpoint

#### Scenario: WebSocket URL for WASM
- **WHEN** user provides a WebSocket URL (e.g., `ws://localhost:8080`)
- **THEN** the connection profile is saved and usable from both native and WASM targets

### Requirement: Edit and delete connection profiles
The system SHALL allow users to edit the configuration of existing connection profiles, including the optional metrics monitoring endpoint, and delete profiles that are no longer needed.

#### Scenario: Edit connection URL
- **WHEN** user edits a disconnected connection profile's server URL
- **THEN** the updated URL is persisted and used on next connect

#### Scenario: Edit metrics endpoint
- **WHEN** user adds, changes, or clears the optional metrics monitoring endpoint on an existing connection profile
- **THEN** the updated metrics configuration is persisted and used for subsequent metrics dashboard access

#### Scenario: Delete connection profile
- **WHEN** user deletes a connection profile
- **THEN** the profile is removed from the list and its persisted data is deleted

### Requirement: Persist connection profiles
The system SHALL persist connection profiles across application restarts. On native platforms, profiles are stored in the platform config directory. On WASM, profiles are stored in browser LocalStorage.

#### Scenario: Restart preserves connections
- **WHEN** user creates connection profiles and restarts the application
- **THEN** all previously saved connection profiles, including any configured metrics monitoring endpoint, are available
