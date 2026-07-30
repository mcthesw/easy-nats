# /// script
# requires-python = ">=3.10"
# dependencies = ["Pillow"]
# ///
"""Convert assets/icons/easy-nats.svg into all required icon formats.

Usage:
    uv run dev/convert_icon.py

Requires Node.js (for @resvg/resvg-js SVG rasterisation).
Generates:
    assets/icons/easy-nats-256.png  – 256px viewport icon (compiled into binary)
    assets/icons/easy-nats.ico      – Windows multi-size ICO (16/32/48/64/128/256)
    packaging/icon.png              – 512px PNG for Linux packaging
    packaging/macos/easy-nats.icns  – padded macOS application icon

The ICNS output requires macOS and its system `iconutil` command.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SVG = ROOT / "assets" / "icons" / "easy-nats.svg"

# Sizes needed for ICO (Windows best practices).
ICO_SIZES = [16, 32, 48, 64, 128, 256]

# All unpadded PNG renders: (width, destination relative to ROOT).
PNG_TARGETS: list[tuple[int, str]] = [
    (512, "packaging/icon.png"),
    (256, "assets/icons/easy-nats-256.png"),
]

MACOS_CANVAS_SIZE = 1024
MACOS_ICON_SIZE = 854
MACOS_ICON_OFFSET = (85, 85)
MACOS_ICONSET_FILES: list[tuple[str, int]] = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
]


def _ensure_resvg_js(tmp: Path) -> Path:
    """Install @resvg/resvg-js into a temp prefix and return the node_modules path."""
    node_modules = tmp / "node_modules"
    if not (node_modules / "@resvg").exists():
        npm = shutil.which("npm")
        if npm is None:
            sys.exit("npm not found – please install Node.js")
        print("Installing @resvg/resvg-js …")
        subprocess.run(
            [npm, "install", "--prefix", str(tmp), "@resvg/resvg-js"],
            check=True,
            stdout=subprocess.DEVNULL,
        )
    return node_modules


def render_svg(svg_path: Path, png_path: Path, width: int, node_modules: Path) -> None:
    """Render *svg_path* to *png_path* at the given *width* using Node + resvg-js."""
    js = f"""\
const {{ Resvg }} = require('@resvg/resvg-js');
const fs = require('fs');
const svg = fs.readFileSync({_js_str(svg_path)});
const resvg = new Resvg(svg, {{ fitTo: {{ mode: 'width', value: {width} }} }});
fs.writeFileSync({_js_str(png_path)}, resvg.render().asPng());
"""
    node = shutil.which("node")
    if node is None:
        sys.exit("node not found – please install Node.js")
    env = {**os.environ, "NODE_PATH": str(node_modules)}
    subprocess.run([node, "-e", js], check=True, env=env)


def build_ico(src_png: Path, dest_ico: Path, sizes: list[int]) -> None:
    """Create a multi-size ICO from a large source PNG using Pillow."""
    img = Image.open(src_png).convert("RGBA")
    # Pillow's ICO writer auto-resizes from the source image.
    img.save(dest_ico, format="ICO", sizes=[(s, s) for s in sizes])


def require_iconutil() -> str:
    """Return the macOS iconutil path or stop before changing generated files."""
    iconutil = shutil.which("iconutil")
    if sys.platform != "darwin" or iconutil is None:
        sys.exit(
            "Cannot regenerate packaging/macos/easy-nats.icns: "
            "macOS and its iconutil command are required."
        )
    return iconutil


def build_macos_icon(node_modules: Path, tmp: Path, iconutil: str) -> None:
    """Build the padded macOS iconset and compile it with iconutil."""
    rendered_icon = tmp / "easy-nats-macos-854.png"
    render_svg(SVG, rendered_icon, MACOS_ICON_SIZE, node_modules)

    with Image.open(rendered_icon) as source:
        canvas = Image.new("RGBA", (MACOS_CANVAS_SIZE, MACOS_CANVAS_SIZE), (0, 0, 0, 0))
        canvas.alpha_composite(source.convert("RGBA"), MACOS_ICON_OFFSET)

    iconset = tmp / "easy-nats.iconset"
    iconset.mkdir()
    for filename, size in MACOS_ICONSET_FILES:
        icon = canvas.resize((size, size), Image.Resampling.LANCZOS)
        icon.save(iconset / filename, format="PNG")

    destination = ROOT / "packaging" / "macos" / "easy-nats.icns"
    destination.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [iconutil, "-c", "icns", str(iconset), "-o", str(destination)],
        check=True,
    )
    print(f"  {destination.relative_to(ROOT)}  (macOS ICNS, 85px transparent margin)")


def _js_str(p: Path) -> str:
    """Return a JS string literal with forward slashes (safe on all OS)."""
    return "'" + p.as_posix().replace("'", "\\'") + "'"


def main() -> None:
    if not SVG.exists():
        sys.exit(f"SVG not found: {SVG}")

    iconutil = require_iconutil()
    tmp = Path(tempfile.gettempdir()) / "easy-nats-icon-build"
    tmp.mkdir(exist_ok=True)
    node_modules = _ensure_resvg_js(tmp)

    # Render PNGs.
    for width, rel in PNG_TARGETS:
        dest = ROOT / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        render_svg(SVG, dest, width, node_modules)
        print(f"  {rel}  ({width}×{width})")

    # Assemble the Windows ICO from the freshly rendered, unpadded 256px icon.
    ico_dest = ROOT / "assets" / "icons" / "easy-nats.ico"
    build_ico(ROOT / "assets" / "icons" / "easy-nats-256.png", ico_dest, ICO_SIZES)
    print(f"  assets/icons/easy-nats.ico  ({', '.join(f'{s}px' for s in ICO_SIZES)})")

    with tempfile.TemporaryDirectory(prefix="easy-nats-macos-icon-") as icon_tmp:
        build_macos_icon(node_modules, Path(icon_tmp), iconutil)

    print("Done.")


if __name__ == "__main__":
    main()
