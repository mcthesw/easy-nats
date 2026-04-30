## Why

easy-nats currently routes most backend operation results through `serde_json::Value`, then stores and reads those values in app state and UI code with string keys. This leaks backend/library payload shapes into UI behavior, as shown by the KV `values` field being displayed as current key count even though it represents stored historical messages.

## What Changes

- Replace backend/app domain operation results with typed Rust structs and event variants.
- Keep JSON only where it is part of the user-facing data domain: payload formatting, JSON Schema validation/templates, protobuf JSON templates, and external monitoring endpoint parsing.
- Migrate by vertical slice: KV, Object Store, Server Info, Stream/Consumer, then command config inputs.
- Remove JSON fallbacks from each migrated domain; unmigrated domains may temporarily continue using the old generic operation result path only until their slice is migrated.
- Split oversized files touched by the migration so domain state and UI code remain bounded and maintainable.

## Capabilities

### New Capabilities
- `backend-operation-contract`: Defines typed backend/app command and result contracts for internal domain data.

### Modified Capabilities
- `workspace-ui`: Resource lists and tab states use typed backend-derived domain models instead of dynamic JSON values.

## Impact

- `crates/nats-backend`: add domain result/config structs, typed backend events, and worker conversions from async-nats types.
- `crates/app/src/app`: update event handling, resource lists, editors, and operation result routing to use typed data.
- `crates/app/src/tabs`: update tab state and renderers to consume typed domain models and split oversized state/UI modules where touched.
- Existing dirty KV display fixes are folded into the typed KV slice and should not leave JSON compatibility fallbacks behind.
