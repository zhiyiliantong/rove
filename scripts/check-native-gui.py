"""Compile the native Linux GUI using a task-specific disposable container.

Run npm ci / npm run build in apps/rove-gui first. Rust dependencies must be
cached: the compiler container has no network. No credential/config directory
is mounted, and no host packages, Docker services or existing containers change.
"""
import argparse
import os
from pathlib import Path
import subprocess
import uuid

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--build-image", action="store_true")
parser.add_argument("--image", default="rove-build-linux:check-20260908")
mode = parser.add_mutually_exclusive_group()
mode.add_argument("--test", action="store_true", help="Link and run native IPC tests instead of check")
mode.add_argument("--build", action="store_true", help="Build the native development GUI binary")
args = parser.parse_args()
if args.build_image:
    subprocess.run([
        "docker", "build", "-t", args.image, "-f",
        str(ROOT / "packaging/linux/Dockerfile.check"), str(ROOT / "packaging/linux"),
    ], check=True)
toolchain = Path(subprocess.check_output([
    "rustup", "which", "--toolchain", "1.96.0", "rustc",
], text=True).strip()).resolve().parent.parent
cache = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).resolve()
assert (ROOT / "apps/rove-gui/dist/index.html").is_file(), "Build the Vue frontend first"
command = ["docker", "run", "--rm", "--name", "rove-native-" + uuid.uuid4().hex[:12], "--network", "none"]
# Keep a workspace target symlink usable when build artifacts live on another
# local volume. Mount only that artifact directory, never its parent directory.
target_cache = (ROOT / "target").resolve()
if not target_cache.is_relative_to(ROOT):
    assert target_cache.is_dir(), "External target cache must exist"
    assert target_cache.name == "target", "Expected a dedicated target artifact directory"
    command.extend(["--mount", f"type=bind,source={target_cache},target={target_cache}"])
for source, target, readonly in [
    (ROOT, ROOT, False),
    (cache / "registry", Path("/root/.cargo/registry"), False),
    (cache / "git", Path("/root/.cargo/git"), True),
    (toolchain, toolchain, True),
]:
    assert source.is_dir(), f"Missing cached build path: {source}"
    command.extend(["--mount", f"type=bind,source={source},target={target}" + (",readonly" if readonly else "")])
command.extend([
    "--env", f"PATH={toolchain}/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    "--workdir", str(ROOT), args.image,
    "cargo", "test" if args.test else "build" if args.build else "check", "--offline", "--locked", "-j", "2", "-p", "rove-gui", "--features", "custom-protocol",
])
if args.test:
    command.append("--lib")
subprocess.run(command, check=True)
