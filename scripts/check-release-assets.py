#!/usr/bin/env python3
"""Validate pinned upstream metadata; optionally verify a downloaded archive.

Never installs, extracts, executes, or updates the manifest from the network.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--asset", help="Manifest filename, required with --archive")
    args = parser.parse_args()
    if bool(args.archive) != bool(args.asset):
        parser.error("--archive and --asset must be supplied together")
    manifest = json.loads((ROOT / "packaging/easytier-assets.json").read_text())
    cargo = tomllib.loads((ROOT / "crates/rove-agent/Cargo.toml").read_text())
    assert manifest["revision"] == cargo["dependencies"]["easytier"]["rev"]
    assert manifest["download_base"] == f'https://github.com/EasyTier/EasyTier/releases/download/v{manifest["version"]}/'
    targets = set()
    assets = {}
    for asset in manifest["assets"]:
        target = (asset["platform"], asset["arch"])
        assert target not in targets and asset["name"] not in assets
        targets.add(target)
        assets[asset["name"]] = asset
        assert re.fullmatch(r"[0-9a-f]{64}", asset["sha256"])
        assert isinstance(asset["size"], int) and 0 < asset["size"] < 256 * 1024 * 1024
        upstream_arch = "arm64" if target == ("windows", "aarch64") else asset["arch"]
        assert asset["name"] == f'easytier-{asset["platform"]}-{upstream_arch}-v{manifest["version"]}.zip'
    assert targets == {(p, a) for p in ("linux", "macos", "windows") for a in ("x86_64", "aarch64")}
    if args.archive:
        if args.asset not in assets:
            parser.error("Unknown pinned asset")
        asset = assets[args.asset]
        with args.archive.open("rb") as stream:
            size = 0
            digest = hashlib.sha256()
            while chunk := stream.read(1024 * 1024):
                size += len(chunk)
                if size > asset["size"]:
                    raise ValueError("Archive exceeds pinned size")
                digest.update(chunk)
        if size != asset["size"] or digest.hexdigest() != asset["sha256"]:
            raise ValueError("Archive does not match pinned SHA-256 and size")
        print(f"PASS: verified archive {args.asset}; not installed or executed")
    print(f"PASS: {len(assets)} pinned EasyTier platform/architecture assets")


if __name__ == "__main__":
    main()
