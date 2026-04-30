## Context

The app already has typed examples for internal data flow: pub/sub messages use `MessageData`, and metrics use `MetricsSnapshot` as a dedicated `BackendEvent`. Most JetStream, KV, Object Store, and Server Info workflows still use `OperationResult { data: serde_json::Value }`, then store those values in app state and read them from UI code with string keys.

That pattern makes field semantics implicit. The KV status bug is the concrete failure mode: `async-nats` exposes `Status::values()` as stored messages including history, but the UI treated a JSON field named `values` as the current key count.

## Goals / Non-Goals

**Goals:**
- Move backend-derived domain data to typed structs and typed `BackendEvent` variants.
- Keep UI/tab state typed for migrated domains.
- Remove JSON field fallback from each migrated domain.
- Keep file sizes bounded while touching large modules.

**Non-Goals:**
- Removing JSON from user payload formatting, JSON Schema, protobuf JSON templates, or external monitoring endpoint parsing.
- Changing user-visible workflows beyond correcting mislabeled data and preserving existing behavior.
- Persisting new data models to disk.

## Decisions

### 1. Use domain-specific typed events

Each migrated domain gets typed event variants instead of adding more cases to `OperationResult(Value)`. This follows the metrics implementation and keeps the compiler responsible for field presence and naming.

Alternatives considered:
- Keep `OperationResult` and deserialize in the app: still leaves a stringly typed boundary.
- One giant `BackendResult` enum with nested JSON payloads: centralizes routing but does not remove the leak.

### 2. Migrate by vertical slice

The implementation order is KV, Object Store, Server Info, Stream/Consumer, then command config inputs. Each slice must finish cleanly before moving on: backend result, app state, and UI rendering for that slice no longer use JSON DTOs.

Alternatives considered:
- Big-bang migration: cleaner final diff but too large to review safely.
- Only fix KV: leaves the same failure pattern elsewhere.

### 3. Typed command configs come after typed results

Result data caused the current bug and has the broadest UI exposure. Command config structs are still important, but they are migrated after result paths are stable so the app can keep compiling through smaller slices.

### 4. JSON is allowed only at explicit dynamic boundaries

Allowed JSON boundaries are payload formatter input/output, JSON Schema documents, protobuf JSON template generation, and parser internals for external JSON APIs. Domain state and UI renderers should not use `serde_json::Value` for backend-derived resource metadata.

## Risks / Trade-offs

- **Large diff surface** -> Migrate by domain and validate each slice before continuing.
- **Temporary old path remains for unmigrated domains** -> Mark tasks so each migrated domain removes its own fallback, then delete or isolate the generic path at the end.
- **File size pressure** -> Split large files when touched, especially tab state and action modules.
- **Semantic drift during conversion** -> Preserve current display behavior with tests before replacing JSON helpers.

## Migration Plan

1. Add OpenSpec requirements and task list.
2. Migrate KV models/events/state/UI and validate.
3. Repeat for Object Store and Server Info.
4. Repeat for Stream/Consumer.
5. Replace command config JSON inputs with typed structs.
6. Remove or isolate the generic JSON operation result and error context path.

Rollback is normal source rollback; no persisted data migration is introduced.
