# Packaging files

This directory holds release-only files.

- `icon.png`: source icon used by packaged builds.
- `easy-nats.desktop`: Linux launcher metadata.
- `homebrew/easy-nats.rb.tmpl`: Homebrew cask template (self-hosted tap).
- `scoop/easy-nats.json.tmpl`: Scoop manifest template for the self-hosted bucket.
- `aur/PKGBUILD.tmpl`: AUR PKGBUILD template.
- `flatpak/*`: Flathub submission files and upstream Flatpak metadata. The manifest template targets tagged release tarballs, and its `x-checker-data` is intended for the generated Flathub app repo after a release with the correct metadata is published.

Generated package artifacts are not committed.

## Linux runtime libraries

Linux release packages are built on Ubuntu 22.04 with cargo-packager 0.11.8.
The X11 backend loads keyboard libraries with `dlopen`, so ELF dependency
scanning alone does not include them in an AppImage. The AppImage explicitly
bundles `libxkbcommon.so.0`, `libxkbcommon-x11.so.0`, and `libxcb-xkb.so.1`,
along with their Ubuntu package copyright notices. The DEB declares
`libxkbcommon-x11-0` as a runtime dependency.

After building an x86_64 AppImage, check it with:

```sh
python3 packaging/verify-appimage.py dist/<filename>.AppImage
```

The script also accepts an extracted AppDir. It checks launcher and application
permissions, bundled keyboard libraries in the launcher's search paths, x86_64
ELF headers, and dependency resolution. It rejects required libraries that
resolve outside the AppDir. Run it only on trusted artifacts: extraction
executes the AppImage runtime.

This check requires `readelf` and `ldd`, but not FUSE. It does not replace
desktop testing: other graphics libraries may resolve from the host, and
keyboard layouts, Compose data, and input methods still depend on the desktop.
Test the final package on a clean supported desktop as well as the intended
X11 or Wayland session.
