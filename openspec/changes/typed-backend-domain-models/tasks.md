## 1. Typed KV Models and Events

- [x] 1.1 Add typed KV bucket, entry, history, key batch, and error context models to `nats-backend`.
- [x] 1.2 Replace backend KV JSON result payloads with typed backend events.
- [x] 1.3 Replace app KV resource lists, tab state, and UI rendering with typed KV models.
- [x] 1.4 Remove KV JSON fallback field reads and keep current key count separate from stored historical value count.
- [x] 1.5 Add or update KV tests and run KV-focused validation.

## 2. Typed Object Store Models and Events

- [x] 2.1 Add typed Object Store bucket, object, transfer result, and error context models.
- [x] 2.2 Replace Object Store backend JSON result payloads with typed backend events.
- [x] 2.3 Replace app Object Store resource lists, tab state, and UI rendering with typed models.
- [x] 2.4 Add or update Object Store tests and run focused validation.

## 3. Typed Server Info Models and Events

- [x] 3.1 Add typed server info and JetStream account info view models.
- [x] 3.2 Replace Server Info backend JSON result payloads with typed backend events.
- [x] 3.3 Replace app Server Info state and UI rendering with typed models.
- [x] 3.4 Add or update Server Info tests and run focused validation.

## 4. Typed Stream and Consumer Models and Events

- [x] 4.1 Add typed stream info, stream message, consumer info, consumer message batch, and stream operation result models.
- [x] 4.2 Replace stream and consumer backend JSON result payloads with typed backend events.
- [x] 4.3 Replace app stream lists, tab state, consumer caches, and UI rendering with typed models.
- [x] 4.4 Update Search Workspace stream snapshots and navigation to consume typed fields.
- [x] 4.5 Add or update stream/consumer/search tests and run focused validation.

## 5. Typed Command Config Inputs

- [x] 5.1 Add typed create/update config structs for streams, consumers, KV buckets, and Object Store buckets.
- [x] 5.2 Update app editors/actions to emit typed config structs instead of `serde_json::Value`.
- [x] 5.3 Update backend workers to convert typed configs into async-nats configs.
- [x] 5.4 Replace `ConsumerEditEditor.original_config` with a typed editable config snapshot.
- [x] 5.5 Add or update command config tests and run focused validation.

## 6. Generic JSON Result Cleanup

- [x] 6.1 Remove or strictly isolate `OperationResult { data: serde_json::Value }`.
- [x] 6.2 Replace `Error { data: Option<serde_json::Value> }` with typed `BackendErrorContext`.
- [x] 6.3 Verify only explicitly dynamic JSON boundaries retain `serde_json::Value`.

## 7. File Size and Full Validation

- [x] 7.1 Split oversized touched files so modified Rust files stay at or below 700 lines.
- [x] 7.2 Run full formatting, tests, clippy, and OpenSpec validation.
