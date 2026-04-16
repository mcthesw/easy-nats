# Roadmap

## Distribution

Current availability and planned publishing targets.

### Available now

| Channel | Install | Auto-update |
|---------|---------|-------------|
| GitHub Releases | [Releases page](https://github.com/mcthesw/easy-nats/releases) | — |
| Homebrew (tap) | `brew install mcthesw/tap/easy-nats` | On each release |
| Scoop (bucket) | `scoop bucket add sworld https://github.com/mcthesw/scoop-bucket && scoop install easy-nats` | On each release |
| APT | `echo "deb [trusted=yes] https://mcthesw.github.io/sworld-apt stable main" \| sudo tee /etc/apt/sources.list.d/mcthesw.list` | On each release |

### Pending setup

| Channel | What's needed | Notes |
|---------|---------------|-------|
| Flathub | Submit initial PR | Manifest and metadata are prepared; updates can be automated later |
| APT (signed) | GPG key pair | Set `APT_GPG_PRIVATE_KEY` + `APT_GPG_PASSPHRASE` secrets; without signing, APT still works but users need `[trusted=yes]` |
| AUR | SSH key pair + AUR account | Deferred for now; keep PKGBUILD template, re-enable publishing after keys are ready |

### Planned

| Channel | Requirement | Status |
|---------|-------------|--------|
| Homebrew Cask (official) | 30 stars + 30 forks | Waiting for eligibility |
| Scoop (Extras) | 100 stars | Waiting for eligibility |
| Flathub | Review on `new-pr` branch | Ready for first submission |
| Debian official | Sponsorship + review | Long-term |
| Fedora / RPM Fusion | Package review | Long-term |

## Features

See [GitHub Issues](https://github.com/mcthesw/easy-nats/issues) for feature requests and bug reports.
