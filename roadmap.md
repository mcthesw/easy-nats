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

### Full-Text Search Workspace

- Add a dedicated in-memory search tab that can aggregate selected KV buckets, stream batches, and Pub/Sub buffers.
- Keep search indexes ephemeral by default; do not persist message or KV content locally unless a separate future change explicitly defines that behavior.
- Evaluate a RAM-backed full-text engine only if simple substring search is no longer sufficient for aggregated search.

## More Ideas

See [GitHub Issues](https://github.com/mcthesw/easy-nats/issues) for feature requests and bug reports.
