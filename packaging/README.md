# Packaging assets

This directory holds source assets used by the release pipeline to build
installable packages for Windows, macOS, and Linux.

## Contents

- `icon.png` — 512×512 PNG used as the canonical application icon source.
  The release workflow converts it into the platform-specific formats
  (`.ico`, `.icns`, and multiple PNG sizes) with ImageMagick. The current
  file is a placeholder (solid teal with an "EN" monogram); replace it with
  a designed icon when one is available — no code or workflow change is
  required as long as the filename stays the same.
- `easy-nats.desktop` — Linux desktop entry template consumed by the `.deb`
  and AppImage packagers to register the app in application menus with
  `Terminal=false`.

## Conventions

- Binary placeholders (e.g. the generated `.ico` / `.icns`) are not committed;
  CI regenerates them from `icon.png` per release.
- Keep everything that is only needed for packaging under this directory so
  the application `crates/app/assets/` tree stays focused on runtime assets.
