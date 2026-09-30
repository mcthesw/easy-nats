#!/usr/bin/env python3
"""Check a trusted, locally built x86_64 AppImage before publishing it.

This catches dlopen libraries that cargo-packager can silently omit. It is an
artifact check, not a substitute for launching on a clean supported desktop.
Requires Linux, readelf (binutils), and ldd (libc-bin); no FUSE is needed.
"""
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

REQUIRED = ("libxkbcommon.so.0", "libxkbcommon-x11.so.0", "libxcb-xkb.so.1")


def check_tree(root):
    root = root.resolve()
    for relative in ("AppRun", "usr/bin/easy-nats"):
        path = root / relative
        mode = path.stat().st_mode & 0o777
        if mode & 0o555 != 0o555:
            raise RuntimeError(f"{relative}: mode {mode:o}; all users need read/execute")

    # These are the x86_64 library directories searched by our AppRun.
    libdirs = [root / p for p in ("usr/lib", "usr/lib/x86_64-linux-gnu", "usr/lib64")]
    libraries = []
    for name in REQUIRED:
        matches = [directory / name for directory in libdirs if (directory / name).is_file()]
        if not matches:
            raise RuntimeError(f"Missing bundled runtime library: {name}")
        for path in matches:
            if not path.resolve().is_relative_to(root):
                raise RuntimeError(f"Library symlink escapes AppDir: {path}")
            libraries.append(path)

    # Other desktop/system libraries may still resolve from the build host.
    # This check does not certify a clean-host runtime or a self-contained bundle.
    env = os.environ.copy()
    env.pop("LD_PRELOAD", None)
    env["LC_ALL"] = "C"
    env["LD_LIBRARY_PATH"] = ":".join(map(str, libdirs))
    for path in [root / "usr/bin/easy-nats", *libraries]:
        header = subprocess.check_output(["readelf", "-h", str(path)], text=True, env=env)
        if "ELF64" not in header or "Advanced Micro Devices X86-64" not in header:
            raise RuntimeError(f"Not an x86_64 ELF: {path}")
        result = subprocess.run(["ldd", str(path)], env=env, text=True,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        print(f"--- {path.relative_to(root)} ---\n{result.stdout}", end="")
        if result.returncode or "not found" in result.stdout:
            raise RuntimeError(f"Unresolved runtime dependency: {path}")
        # A preinstalled build-host library must not hide a broken bundled link.
        for name, resolved in re.findall(r"^\s*(\S+) => (\S+)", result.stdout, re.M):
            if name in REQUIRED and not Path(resolved).resolve().is_relative_to(root):
                raise RuntimeError(f"{name} resolved outside AppDir: {resolved}")
    print("AppImage permissions, required ELF libraries, and dependency resolution passed")


def main():
    if len(sys.argv) != 2:
        raise RuntimeError("Usage: verify-appimage.py <trusted.AppImage|extracted-AppDir>")
    source = Path(sys.argv[1]).resolve()
    if source.is_dir():
        check_tree(source)
    else:
        with tempfile.TemporaryDirectory(prefix="verify-appimage-") as temp:
            subprocess.run([str(source), "--appimage-extract"], cwd=temp,
                           stdout=subprocess.DEVNULL, check=True)
            check_tree(Path(temp) / "squashfs-root")


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        sys.exit(f"::error::{error}")
