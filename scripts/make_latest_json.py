#!/usr/bin/env python3
"""Builds the `latest.json` update manifest attached to each GitHub release.

One file serves two updaters:

* `cargo-packager-updater` (this app) wants, per platform: url, signature and a
  `format` field.
* `tauri-plugin-updater` (the previous Tauri build, still installed on users'
  machines) wants url and signature, and ignores unknown fields such as `format`.

Both look the platform up as `<os>-<arch>`; Tauri also tries
`<os>-<arch>-<bundle>` first, so both keys are written.

Usage:
    make_latest_json.py --version 0.1.7 --repo hebus/pr-dashboard \
        --installer dist/pr-dashboard_0.1.7_x64-setup.exe \
        --signature dist/pr-dashboard_0.1.7_x64-setup.exe.sig \
        --out dist/latest.json [--notes "..."] [--pub-date 2026-11-11T10:00:00Z]
"""

import argparse
import json
import pathlib
import sys
from datetime import datetime, timezone
from urllib.parse import quote


def build_manifest(version, repo, installer_name, signature, notes, pub_date):
    tag = f"v{version}"
    entry = {
        "signature": signature,
        "url": f"https://github.com/{repo}/releases/download/{tag}/{quote(installer_name)}",
        "format": "nsis",
    }
    return {
        "version": version,
        "notes": notes,
        "pub_date": pub_date,
        "platforms": {
            "windows-x86_64": entry,
            "windows-x86_64-nsis": dict(entry),
        },
    }


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--version", required=True, help="semver without the leading v")
    p.add_argument("--repo", required=True, help="owner/name of the GitHub repository")
    p.add_argument("--installer", required=True, type=pathlib.Path)
    p.add_argument("--signature", required=True, type=pathlib.Path, help="the .sig file of the installer")
    p.add_argument("--out", required=True, type=pathlib.Path)
    p.add_argument("--notes", default="")
    p.add_argument("--pub-date", default=None, help="RFC 3339; defaults to now (UTC)")
    args = p.parse_args(argv)

    if args.version.startswith("v"):
        p.error("--version must not start with 'v'")
    if not args.installer.name.endswith(".exe"):
        p.error(f"--installer must be the NSIS .exe, got {args.installer.name}")

    # The .sig file holds the base64 text as-is; a trailing newline is not part of it.
    signature = args.signature.read_text(encoding="utf-8").strip()
    if not signature:
        p.error(f"{args.signature} is empty")

    pub_date = args.pub_date or datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    manifest = build_manifest(
        args.version, args.repo, args.installer.name, signature, args.notes, pub_date
    )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {args.out} for v{args.version}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
