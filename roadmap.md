# Roadmap

## Distribution

| Channel | Goal | Status |
|---------|------|--------|
| APT (signed) | Publish signed APT repository metadata so users do not need `[trusted=yes]` | Planned hardening |
| AUR | Publish an Arch package from the retained PKGBUILD template | Deferred until AUR account and SSH key setup are ready |
| Homebrew Cask (official) | Submit to the official Homebrew Cask repository | Waiting for upstream eligibility |
| Scoop Extras | Submit to the Scoop Extras bucket | Waiting for upstream eligibility |
| Debian official | Package through Debian review and sponsorship | Long-term |
| Fedora / RPM Fusion | Package through Fedora or RPM Fusion review | Long-term |

## Search

### In-Memory Search Workspace

- Delivered a dedicated in-memory search tab that aggregates user-selected open KV buckets, stream batches, and Pub/Sub buffers.
- Keep search state ephemeral: selected sources, cached results, and fetched KV values stay in memory and are not persisted locally.
- Keep KV value search explicit and bounded through "scan more values" batches instead of silently loading entire buckets.

### Future Full-Text Engine

- Evaluate a RAM-backed full-text engine only if simple substring search becomes insufficient for aggregated search.

## More Ideas

See [GitHub Issues](https://github.com/mcthesw/easy-nats/issues) for feature requests and bug reports.
