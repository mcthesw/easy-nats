## Purpose
Support reliable connect and disconnect behavior that follows user intent rather than transient connection state.

## Requirements

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

### Requirement: Authentication methods
The system SHALL support the following auth methods: no auth, token, username/password, NKey, credentials file (.creds), and TLS client certificate.

#### Scenario: Token authentication
- **WHEN** user configures a connection with a token
- **THEN** the system connects to the NATS server using the provided token

#### Scenario: NKey authentication
- **WHEN** user configures a connection with an NKey seed
- **THEN** the system signs the server nonce with the NKey and authenticates

#### Scenario: Credentials file authentication
- **WHEN** user selects a `.creds` file path (native) or pastes its content (WASM)
- **THEN** the system uses the JWT and NKey from the credentials to authenticate

### Requirement: Connect and disconnect
The system SHALL allow users to connect to and disconnect from a saved connection profile at any time. The system SHALL separate user intent (wants connected vs. wants disconnected) from transient connection status (Connected, Disconnected, Connecting, Error). When the user clicks "Disconnect", the backend SHALL fully tear down the NATS client, preventing any automatic reconnection. The sidebar button SHALL reflect user intent, not transient NATS status events.

#### Scenario: Successful connection
- **WHEN** user clicks "Connect" on a valid connection profile
- **THEN** the system establishes a NATS connection, the status changes to "Connected", and the button shows "Disconnect"

#### Scenario: Connection failure
- **WHEN** user clicks "Connect" but the server is unreachable
- **THEN** the system displays an error message with the failure reason and the button remains "Connect"

#### Scenario: Disconnect
- **WHEN** user clicks "Disconnect" on an active connection
- **THEN** the system fully tears down the NATS client, cleans up subscriptions, prevents auto-reconnect, and the button shows "Connect"

#### Scenario: Server drops mid-session
- **WHEN** the NATS server becomes unreachable while connected
- **THEN** the status indicator shows the error/disconnected state, BUT the button continues to show "Disconnect" and the async_nats client attempts automatic reconnection

#### Scenario: User disconnects during server outage
- **WHEN** the NATS server is unreachable and the user clicks "Disconnect"
- **THEN** the backend fully destroys the client, auto-reconnection stops, and the button shows "Connect"

#### Scenario: Server recovers after temporary outage
- **WHEN** the NATS server comes back online after a temporary disconnect and the user has NOT clicked "Disconnect"
- **THEN** the async_nats client reconnects automatically and the status returns to "Connected"

### Requirement: Multiple simultaneous connections
The system SHALL support multiple active NATS connections at the same time, each operating independently.

#### Scenario: Two connections active
- **WHEN** user connects to Server A and Server B simultaneously
- **THEN** both connections are active and user can interact with each independently

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
