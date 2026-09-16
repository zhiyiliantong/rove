"""Build a local Linux runtime .deb without installing or starting anything.

Development packaging only; supported runtime baseline must be verified on the
destination OS. The separate Tauri GUI package depends on this runtime package.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--arch", choices=["x86_64", "aarch64"], default="x86_64")
    parser.add_argument("--profile", choices=["dev", "release"], default="dev")
    parser.add_argument("--output", type=Path, default=ROOT / "target/packages")
    args = parser.parse_args()
    manifest = json.loads((ROOT / "packaging/easytier-assets.json").read_text())
    asset = next(a for a in manifest["assets"] if a["platform"] == "linux" and a["arch"] == args.arch)
    subprocess.run(["python3", str(ROOT / "scripts/check-release-assets.py"),
                    "--archive", str(args.archive), "--asset", asset["name"]], check=True)
    # Do not silently use an old/default-feature agent binary.
    target = f"{args.arch}-unknown-linux-gnu"
    host = subprocess.check_output(["rustc", "-vV"], text=True).split("host: ", 1)[1].splitlines()[0]
    target_flags = [] if host == target else ["--target", target]
    subprocess.run(["cargo", "build", "--offline", "--locked", *target_flags,
                    "--profile", args.profile, "-p", "rove-agent", "-p", "rove-cli",
                    "--features", "rove-agent/easytier", "--bins"], cwd=ROOT, check=True)
    profile = "debug" if args.profile == "dev" else "release"
    machine = {"x86_64": 62, "aarch64": 183}[args.arch]
    deb_arch = {"x86_64": "amd64", "aarch64": "arm64"}[args.arch]
    args.output.mkdir(parents=True, exist_ok=True)
    output = args.output.resolve() / f"rove-runtime_0.1.0_{deb_arch}.deb"
    if output.exists():
        raise FileExistsError(f"Refusing to replace existing package: {output}")
    with tempfile.TemporaryDirectory(prefix="rove-deb-") as temporary:
        stage = Path(temporary)
        root = stage / "root"

        def install(data, destination, mode=0o644):
            path = root / destination.lstrip("/")
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            path.chmod(mode)

        def executable(data, destination):
            if len(data) < 20 or data[:6] != b"\x7fELF\x02\x01" or struct.unpack_from("<H", data, 18)[0] != machine:
                raise ValueError(f"Wrong ELF architecture: {destination}")
            install(data, destination, 0o755)

        for name, destination in [("rove-agent", "/usr/lib/rove/rove-agent"), ("rove", "/usr/bin/rove")]:
            build_dir = ROOT / "target" if host == target else ROOT / "target" / target
            executable((build_dir / profile / name).read_bytes(), destination)
        with zipfile.ZipFile(args.archive) as archive:
            for name in ["easytier-core", "easytier-cli"]:
                # Only the two exact manifest-selected members are read. No
                # extractall, symlinks, archive paths or web service installed.
                member = f"easytier-linux-{args.arch}/{name}"
                info = archive.getinfo(member)
                if info.file_size > 128 * 1024 * 1024:
                    raise ValueError("Oversized upstream executable")
                executable(archive.read(info), f"/usr/lib/rove/{name}")
        for name, folder in [("rove-agent.service", "user"), ("rove-easytier.service", "system")]:
            install((ROOT / "packaging/linux" / name).read_bytes(), f"/usr/lib/systemd/{folder}/{name}")
        install((ROOT / "docs/linux-packaging.md").read_bytes(), "/usr/share/doc/rove-runtime/README.md")
        install((json.dumps({"profile": args.profile, "target": target, "easytier": asset}, indent=2) + "\n").encode(),
                "/usr/share/doc/rove-runtime/build.json")
        control = f"""Package: rove-runtime
Version: 0.1.0
Architecture: {deb_arch}
Maintainer: Rove developers
Section: net
Priority: optional
Depends: libc6 (>= 2.39), libgcc-s1, ca-certificates, systemd
Description: Rove agent, CLI and dedicated EasyTier runtime
 Development package; activation is explicit and user data is preserved.
"""
        install(control.encode(), "/DEBIAN/control")
        root.chmod(0o755)
        for directory in root.rglob("*"):
            if directory.is_dir():
                directory.chmod(0o755)
        subprocess.run(["dpkg-deb", "--root-owner-group", "--build", str(root), str(output)], check=True)
    print(f"Built {output} (profile={args.profile}); not installed, enabled or platform-certified")
    print("SHA-256: " + hashlib.sha256(output.read_bytes()).hexdigest())


if __name__ == "__main__":
    main()
