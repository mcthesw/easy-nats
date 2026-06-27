# Easy NATS

Manage [NATS](https://nats.io/) messaging, JetStream data, object storage, and server metrics in one workspace.

![main-menu](assets/imgs/main-menu.png)

## Features

- **Connections and Messaging** — Save multiple server profiles, publish with headers or request-reply, and subscribe to live subjects with wildcard support.
- **Streams, KV, and Object Store** — Browse stream messages, inspect KV revision history, and upload, download, or delete object store files.
- **Metrics Dashboard** — Track connection health, summary stats, and live plots for message rate, traffic, and JetStream usage.
- **Message Inspection** — Auto-detect JSON, text, and binary payloads with pretty JSON, hex dump, Base64, and manual format override.
- **Dockable Workspace** — Arrange publishers, subscribers, resources, and detail panes in a flexible multi-tab layout.
- **Themes** — Choose from egui dark, egui light, Catppuccin Latte, Frappé, Macchiato, and Mocha.

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
