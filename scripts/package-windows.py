"""Package already-built Windows GNU agent/CLI for native smoke tests.

This is a portable development package, not the complete desktop installer or
the Windows overlay/network-service distribution.
"""
from pathlib import Path
import argparse
import hashlib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--output", type=Path, default=ROOT / "target/packages/rove-windows-x86_64-dev.zip")
parser.add_argument("--gui", action="store_true", help="Include the prebuilt Tauri GUI")
args = parser.parse_args()
args.output.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(args.output, "x", compression=zipfile.ZIP_DEFLATED) as package:
    for name in ["rove.exe", "rove-agent.exe"] + (["rove-gui.exe", "WebView2Loader.dll"] if args.gui else []):
        path = ROOT / "target/x86_64-pc-windows-gnu/debug" / name
        with path.open("rb") as stream:
            if stream.read(2) != b"MZ":
                raise ValueError(f"Not a Windows executable: {name}")
        package.write(path, name)
    package.write(ROOT / "packaging/windows/Test-Rove.ps1", "Test-Rove.ps1")
    if args.gui:
        package.write(ROOT / "packaging/windows/Test-RoveGui.ps1", "Test-RoveGui.ps1")
    package.writestr("README.txt", "Rove Windows x86_64 portable development package.\nIncludes CLI and agent" + (" and Tauri GUI (requires WebView2)" if args.gui else "") + ". No Windows overlay/network service.\nRun powershell -NoProfile -File .\\Test-Rove.ps1 from a user-owned test directory.\nFor GUI use, start rove-agent.exe, then rove-gui.exe as the same user.\nData remains after tests. No global PATH, service or firewall modifications.\n")
print(args.output)
print("SHA-256: " + hashlib.sha256(args.output.read_bytes()).hexdigest())
