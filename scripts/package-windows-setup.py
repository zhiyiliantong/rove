"""Build the Rove Windows per-user installer (NSIS) from cross-compiled artifacts.

This development installer includes GUI, CLI, the current-user agent, a restricted
EasyTier network helper and the pinned signed Wintun DLL. Network startup uses
normal Windows elevation for the helper, never the business agent. Installation
does not change PATH, proxy or physical adapters. User data survives uninstall.
"""
from pathlib import Path
import argparse
import hashlib
import ipaddress
import json
import shutil
import subprocess
import sys
import tempfile
import zipfile
from urllib.parse import urlsplit
from frontend_assets import validate_frontend_assets

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE = ROOT / "target/x86_64-pc-windows-gnu/debug"
DEFAULT_ICON = ROOT / "apps/rove-gui/src-tauri/icons/icon.ico"
SCRIPT = ROOT / "packaging/windows/rove.nsi"
BUILT = ["rove.exe", "rove-agent.exe", "rove-network-helper.exe", "rove-gui.exe", "WebView2Loader.dll"]
SCRIPTS = ["Test-Rove.ps1", "Test-RoveGui.ps1"]

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
parser.add_argument("--output", type=Path,
                    default=ROOT / "target/packages/rove-windows-x86_64-setup.exe")
parser.add_argument("--version", default="0.1.0")
parser.add_argument("--icon", type=Path, default=DEFAULT_ICON)
parser.add_argument("--wintun-archive", type=Path, required=True,
                    help="official signed wintun-0.14.1.zip, verified against the published SHA-256")
parser.add_argument("--windivert-archive", type=Path, required=True,
                    help="official WinDivert-2.2.2-A.zip for Windows FakeTCP")
parser.add_argument("--windivert-source", type=Path, required=True,
                    help="official WinDivert-2.2.2-Source.zip, retained with its license")
parser.add_argument(
    "--discovery-service-url",
    default="http://10.1.2.237:8090",
    help="packaged public-node directory default; use an empty value to omit it",
)
args = parser.parse_args()

# Asset keys remain uncompressed in Tauri's embedded asset lookup table.
# Reject empty/stale frontend bundles before producing an installable package.
try:
    validate_frontend_assets(ROOT / "apps/rove-gui/dist/index.html", args.source / "rove-gui.exe")
except ValueError as error:
    sys.exit(str(error))


def validate_discovery_service_url(value: str) -> str:
    if not value:
        return value
    parsed = urlsplit(value)
    if (parsed.scheme not in ("http", "https") or not parsed.hostname
            or parsed.username or parsed.password or parsed.query or parsed.fragment):
        sys.exit("invalid discovery service URL")
    if parsed.scheme == "http":
        try:
            address = ipaddress.ip_address(parsed.hostname)
        except ValueError:
            sys.exit("HTTP discovery defaults must use a private or loopback IP address")
        if not (address.is_private or address.is_loopback):
            sys.exit("HTTP discovery defaults must use a private or loopback IP address")
    return value.rstrip("/")


discovery_service_url = validate_discovery_service_url(args.discovery_service_url.strip())

if not args.wintun_archive.is_file():
    sys.exit("missing official Wintun archive")
if hashlib.sha256(args.wintun_archive.read_bytes()).hexdigest() != "07c256185d6ee3652e09fa55c0b673e2624b565e02c4b9091c79ca7d2f24ef51":
    sys.exit("official Wintun archive checksum mismatch")
with zipfile.ZipFile(args.wintun_archive) as archive:
    wintun_dll = archive.read("wintun/bin/amd64/wintun.dll")
    wintun_license = archive.read("wintun/LICENSE.txt")
if hashlib.sha256(wintun_dll).hexdigest() != "e5da8447dc2c320edc0fc52fa01885c103de8c118481f683643cacc3220dafce":
    sys.exit("Wintun DLL does not match the pinned runtime")

for path, checksum in [
    (args.windivert_archive, "63cb41763bb4b20f600b6de04e991a9c2be73279e317d4d82f237b150c5f3f15"),
    (args.windivert_source, "8e904fbeef2a6180ffc533e9b636afd14282af7510e421a9cb381d978302acd9"),
]:
    if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != checksum:
        sys.exit("official WinDivert input missing or checksum mismatch")
with zipfile.ZipFile(args.windivert_archive) as archive:
    windivert_driver = archive.read("WinDivert-2.2.2-A/x64/WinDivert64.sys")
    windivert_license = archive.read("WinDivert-2.2.2-A/LICENSE")
if hashlib.sha256(windivert_driver).hexdigest() != "8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2":
    sys.exit("WinDivert driver does not match the pinned runtime")

makensis = shutil.which("makensis")
if makensis is None:
    sys.exit("makensis (NSIS) is required; install the nsis package")
if args.output.exists():
    sys.exit(f"refusing to overwrite existing package: {args.output}")
if not args.icon.is_file():
    sys.exit(f"missing icon: {args.icon}")

for name in BUILT:
    path = args.source / name
    if not path.is_file():
        sys.exit(f"missing build artifact: {path}")
    if path.read_bytes()[:2] != b"MZ":
        sys.exit(f"not a Windows PE file: {path}")
    imports = subprocess.run(["x86_64-w64-mingw32-objdump", "-p", str(path)],
                             text=True, capture_output=True, check=True).stdout.lower()
    if "dll name: packet.dll" in imports or "dll name: wpcap.dll" in imports:
        sys.exit(f"unlicensed Npcap runtime dependency in {name}")
for name in SCRIPTS:
    if not (ROOT / "packaging/windows" / name).is_file():
        sys.exit(f"missing regression script: {name}")

args.output.parent.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="rove-windows-setup-") as scratch:
    stage = Path(scratch)
    for name in BUILT:
        shutil.copy2(args.source / name, stage / name)
    for name in SCRIPTS:
        shutil.copy2(ROOT / "packaging/windows" / name, stage / name)
    (stage / "wintun.dll").write_bytes(wintun_dll)
    (stage / "WINTUN-LICENSE.txt").write_bytes(wintun_license)
    (stage / "WinDivert64.sys").write_bytes(windivert_driver)
    (stage / "WINDIVERT-LICENSE.txt").write_bytes(windivert_license)
    shutil.copy2(args.windivert_source, stage / "WinDivert-2.2.2-Source.zip")
    (stage / "rove-defaults.json").write_text(
        json.dumps({"discovery_service_url": discovery_service_url or None}, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    command = [makensis,
               f"-DVERSION={args.version}",
               f"-DSRCDIR={stage}",
               f"-DOUTFILE={args.output.resolve()}",
               f"-DICONFILE={args.icon.resolve()}",
               f"-DDEFAULTDISCOVERYURL={discovery_service_url}",
               str(SCRIPT)]
    result = subprocess.run(command, text=True, capture_output=True)
sys.stdout.write(result.stdout)
sys.stderr.write(result.stderr)
if result.returncode != 0:
    sys.exit(f"makensis failed with exit code {result.returncode}")
if not args.output.is_file():
    sys.exit(f"installer was not produced: {args.output}")
print(args.output)
print("bytes: " + str(args.output.stat().st_size))
print("SHA-256: " + hashlib.sha256(args.output.read_bytes()).hexdigest())
