# Icons

![Easy NATS icon](./easy-nats.svg)

`easy-nats.svg` is the single design source for all application icons. Do not edit
the generated PNG, ICO, or ICNS files directly.

Windows and Linux use unpadded renders of the complete SVG:

- `assets/icons/easy-nats-256.png`
- `assets/icons/easy-nats.ico`
- `packaging/icon.png`

macOS uses `packaging/macos/easy-nats.icns`. Its 1024×1024 source image places
an 854×854 render of the complete SVG at `(85, 85)`, leaving 85 pixels of
transparent padding on every side.

After updating the SVG, regenerate all app and package icons on macOS with:

```sh
uv run dev/convert_icon.py
```

The script uses the macOS system `iconutil` command to create the ICNS file, so
the ICNS cannot be regenerated on other platforms.
