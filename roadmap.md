# Roadmap

## Distribution

Current availability and planned publishing targets.

### Available now

| Channel | Install | Auto-update |
|---------|---------|-------------|
| GitHub Releases | [Releases page](https://github.com/mcthesw/easy-nats/releases) | — |
| Homebrew (tap) | `brew install mcthesw/tap/easy-nats` | On each release |
| APT | See [packaging/README.md](packaging/README.md) | On each release |
| Flathub | `flatpak install flathub io.github.mcthesw.easy_nats` | FEDC bot |

### Pending setup

| Channel | What's needed | Notes |
|---------|---------------|-------|
| Scoop (Extras) | Star count ≥ 100 | Manifest ready, Excavator handles auto-updates |
| AUR | SSH key pair + AUR account | Register at https://aur.archlinux.org, add SSH key, set `AUR_SSH_PRIVATE_KEY` / `AUR_USERNAME` / `AUR_EMAIL` secrets |
| APT (signed) | GPG key pair | Set `APT_GPG_PRIVATE_KEY` + `APT_GPG_PASSPHRASE` secrets; without signing, APT still works but users need `[trusted=yes]` |

### Planned

| Channel | Requirement | Status |
|---------|-------------|--------|
| Homebrew Cask (official) | 30 stars + 30 forks | Waiting for eligibility |
| Scoop (Extras) | 100 stars | Waiting for eligibility |
| Debian official | Sponsorship + review | Long-term |
| Fedora / RPM Fusion | Package review | Long-term |

## Features

See [GitHub Issues](https://github.com/mcthesw/easy-nats/issues) for feature requests and bug reports.
