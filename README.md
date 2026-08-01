# Easy NATS

**A fast, native desktop workspace for [NATS](https://nats.io/).** It starts
quickly, stays responsive under live traffic, and keeps search, publishers,
subscribers, resources, and payload inspection in one workspace. Built in Rust,
without an Electron runtime.

[Try the interactive preview](https://easy-nats.sworld.club) · [Download Easy NATS](https://github.com/mcthesw/easy-nats/releases)

![Easy NATS with multiple windows, live metrics, a publisher, and a subscriber](assets/imgs/main-hero.png)

## A workspace for real NATS work

### Search everything

Search across the open workspace, including retained Stream messages, KV keys
and values, and live Subscriber messages from multiple connections. Every result
keeps its source and opens back at the original message or value instead of
leaving you to hunt for it again.

![Search Stream messages, KV entries, and Subscriber traffic as you type](assets/demos/search-everything.gif)

### Work in parallel

Keep publishers, subscribers, resource browsers, message details, and server
metrics open together. Dock them into a focused layout or float a window when
you need to compare live traffic with the data you are editing.

![Publish, subscribe, inspect messages, and watch metrics in parallel](assets/demos/work-in-parallel.gif)

### Inspect real payloads

Read JSON, plain text, binary, Base64, and hex without leaving the app. Start
with automatic detection, choose a format when you need control, and decode
structured messages with JSON Schema or Protobuf definitions.

![Inspect a real payload with automatic JSON detection and a hex dump](assets/demos/inspect-real-payloads.gif)

Easy NATS also keeps the surrounding work close: saved server connections,
request-reply with headers and wildcards, JetStream Streams and consumers, KV
history, Object Store file operations, and live connection and JetStream
metrics.

## Install

Pre-built binaries are available on the [Releases](https://github.com/mcthesw/easy-nats/releases) page.

**Windows (Scoop)**

```powershell
scoop bucket add sworld https://github.com/mcthesw/scoop-bucket
scoop install easy-nats
```

**macOS (Homebrew)**

```bash
brew install --cask mcthesw/tap/easy-nats
xattr -dr com.apple.quarantine "/Applications/Easy NATS.app"
```

Upgrade:

```bash
brew upgrade --cask mcthesw/tap/easy-nats
xattr -dr com.apple.quarantine "/Applications/Easy NATS.app"
```

**Linux (APT)**

```bash
echo "deb [trusted=yes] https://mcthesw.github.io/sworld-apt stable main" | \
  sudo tee /etc/apt/sources.list.d/mcthesw.list
sudo apt update
sudo apt install easy-nats
```

**Linux (Flathub)**

[![Download on Flathub](https://flathub.org/assets/badges/flathub-badge-en.svg)](https://flathub.org/zh-Hans/apps/io.github.mcthesw.easy-nats)

For `.rpm` and `.AppImage`, use the [Releases](https://github.com/mcthesw/easy-nats/releases) page.

See [roadmap.md](roadmap.md) for additional distribution channels (AUR, etc.).

## Build

Requires **Rust 2024 edition** (rustc 1.92+).

```bash
cargo build --release
```

The binary is output to `target/release/easy-nats` (or `easy-nats.exe` on Windows).

## Development

Run the desktop app from the workspace root:

```bash
cargo run -p easy-nats
```

### Local NATS sandbox

The repository includes a Docker Compose setup with two local NATS servers:

| Server | Client URL | Monitoring URL | Auth |
|--------|------------|----------------|------|
| `nats` | `nats://localhost:4222` | `http://localhost:8222` | token `dev-secret-token` |
| `nats-open` | `nats://localhost:4223` | `http://localhost:8223` | none |

Start the sandbox:

```bash
docker compose -f dev/docker-compose.yml up -d
```

Seed JetStream streams, consumers, KV buckets and sample messages on the open server:

```bash
bash dev/seed.sh
```

Generate continuous traffic for publisher, subscriber, stream and metrics testing:

```bash
bash dev/traffic.sh
```

The seed and traffic scripts require the `nats` CLI on the host and target
`nats://localhost:4223`.

Stop the sandbox when finished:

```bash
docker compose -f dev/docker-compose.yml down
```

## License

[MIT](LICENSE)
