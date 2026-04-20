# Packaging files

This directory holds release-only files.

- `icon.png`: source icon used by packaged builds.
- `easy-nats.desktop`: Linux launcher metadata.
- `homebrew/easy-nats.rb.tmpl`: Homebrew cask template (self-hosted tap).
- `scoop/easy-nats.json.tmpl`: Scoop manifest template for the self-hosted bucket.
- `aur/PKGBUILD.tmpl`: AUR PKGBUILD template.
- `flatpak/*`: Flathub submission files and upstream Flatpak metadata. The manifest template targets tagged release tarballs, and its `x-checker-data` is intended for the generated Flathub app repo after a release with the correct metadata is published.

Generated package artifacts are not committed.
