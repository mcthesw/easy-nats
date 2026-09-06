# Desktop keyboard regression checks

Use an isolated app configuration and disposable NATS data. Keyboard tests live
in `crates/app/src/keyboard/tests.rs`; command targeting tests are in
`crates/app/src/app/commands.rs`. They use real egui input frames, including
press/release pairs, held keys, IME events, and text plus Enter in one frame.

## Automated checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all
```

## Interactive environment

```sh
docker compose -f dev/docker-compose.yml up -d
XDG_CONFIG_HOME=/tmp/easy-nats-47/config \
XDG_DATA_HOME=/tmp/easy-nats-47/data \
XDG_CACHE_HOME=/tmp/easy-nats-47/cache \
XDG_STATE_HOME=/tmp/easy-nats-47/state \
EGUI_INSPECTION=1 cargo run -p easy-nats
```

Connect the app to `nats://127.0.0.1:4223`. The other Compose server uses port
4222 and the test token from `dev/nats-server.conf`. Prepare streams using
`dev/seed.sh`, or run equivalent NATS CLI commands through the `nats-box`
service. `dev/seed-inside.sh` seeds entries but does not create the streams.

Attach egui MCP to `127.0.0.1:5719`. Use `query_tree` to locate current widget IDs,
`type_text` / `press_key` to drive them, `wait_for` to let sizing passes settle,
and `screenshot` to capture the visible app. Re-query IDs after reopening tabs.

The MCP tool must be compatible with egui 0.35. A tool built against egui 0.36
may send `ModifiersChanged`, which 0.35 rejects. For the September 2026 local
verification, a temporary MCP build omitted those markers while retaining the
modifiers carried on Key/PointerButton events. The app dependencies were unchanged.

## Scenarios

- New connection: first-field focus, dynamic auth fields, forward/reverse Tab,
  Enter save, Esc cancel, focus restoration after both save and cancel.
- Subject history: no initial selection; arrows select; Enter only completes;
  second Enter submits; Esc stays closed; Tab exits; typing or pasting plus Enter
  in one frame never accepts an outdated candidate.
- Publisher: Enter in payload adds a newline, Ctrl/Cmd+Enter publishes once,
  Ctrl/Cmd+Shift+Enter requests once. Verify the subscriber count and response.
- Subscriber reply, Stream publish, KV save: multiline submission uses current
  field contents. Verify stored KV bytes, not only a success toast.
- Multiple Schema forms: shortcuts target the focused form. Real-time searches
  and purge fields never acquire a save/delete action.
- Parallel windows: a background search cannot submit an editor. Modal
  confirmations start on Cancel; Esc exits only the innermost interaction.
- Palette: English/Chinese queries, unavailable reasons, original form target,
  revalidation after connection loss, focus restoration, keyboard and mouse use.
- Tabs: close through existing unsubscribe/guard cleanup; navigation stays in
  the focused split; a disconnected current connection never falls back to a
  different connected server selected in the sidebar.
- Visuals: English/Chinese, light/dark themes, 900×600 or larger windows; no
  clipped shortcut hints, clear selection, aligned subject suggestions.

## Local verification notes (2026-09-06)

The automated suite passed 226 tests. Linux desktop checks used egui MCP and
real NATS traffic: connection entry, candidate acceptance, publish/request, KV
byte verification, palette focus restoration, and safe deletion cancellation.
Screenshots were captured at 1024×600; the compositor did not honor the requested
900-pixel resize. System Docker socket access was denied; the rootless attempt
also lacked `newuidmap`. Compose was attempted but did not run. Temporary native
NATS 2.14.6 servers on the equivalent ports supplied the interactive test data.

MCP `type_text` injects Text events, not a real IME composition. Composition
protection is covered by synthetic egui IME tests; real input-method and
Windows/macOS desktop checks still require those environments.

Presentation screenshots are kept outside the repository under
`/tmp/easy-nats-47/showcase/`; intermediate captures and tool logs are in its
parent directory. No test services or inspection port are enabled by a normal
application launch.


## Tab switcher follow-up (2026-09-06)

- Searchable tabs span all splits and include connection names and subjects.
  Verify typing a subject and Enter in one frame uses the new query; an empty
  result must not activate an unrelated tab.
- Ctrl+Tab snapshots the focused split in most-recently-used order. Keep Ctrl
  held to cycle; Shift reverses. Releasing Ctrl commits. Escape (including while
  Ctrl is held) and losing application focus cancel without changing the tab.
- Verify two quick switches alternate between publisher and subscriber, mouse
  tab selections update recency, and closed/moved tabs cannot become stale targets.
- Palette groups are not selectable rows. Filter across groups and confirm the
  first enabled action; scroll to Application to reach Settings, Logs and Schemas.
- Native Linux X11 verification used the same egui MCP plus XTest key events
  targeting the app window, with separate key-down/key-up events. This verifies
  real modifier state, which the temporary egui 0.35-compatible MCP press_key
  helper cannot hold by itself. Held Ctrl, release confirmation, repeated
  publisher/subscriber switching and Escape cancellation were checked.
- Updated Chinese/light and English/dark captures were visually inspected.
  The rejected translucent palette captures have been deleted. Use the current
  index in `/tmp/easy-nats-47/showcase/README.md` for presentation files.
