#!/usr/bin/env python3
"""Exercise only a fresh, loopback-only EasyTier process, never an existing service.

First build: cargo build --locked -p rove-agent --features easytier --example easytier_probe
"""
import os
import argparse
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--probe", choices=["easytier_probe", "network_lifecycle_probe"], default="easytier_probe")
    args = parser.parse_args()
    core = shutil.which("easytier-core")
    probe = Path(__file__).resolve().parents[1] / "target/debug/examples" / args.probe
    if not core or not probe.is_file():
        raise SystemExit("Install the pinned easytier-core and build easytier_probe first")
    version = subprocess.check_output([core, "--version"], text=True).strip()
    if version != "easytier-core 2.6.4-8428a89d":
        raise SystemExit("EasyTier version does not match the pinned adapter")
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    # No inherited ET_* configuration or user configuration directory.
    environment = {k: v for k, v in os.environ.items() if not k.startswith("ET_")}
    with tempfile.TemporaryDirectory(prefix="rove-easytier-probe-") as directory:
        process = subprocess.Popen(
            [core, "--daemon", "--config-dir", directory,
             "--rpc-portal", f"127.0.0.1:{port}",
             "--rpc-portal-whitelist", "127.0.0.1/32",
             "--console-log-level", "error"],
            cwd=directory, env=environment,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        try:
            deadline = time.monotonic() + 10
            while True:
                if process.poll() is not None:
                    raise RuntimeError("Isolated EasyTier exited before readiness")
                try:
                    with socket.create_connection(("127.0.0.1", port), timeout=.2):
                        break
                except OSError:
                    if time.monotonic() >= deadline:
                        raise RuntimeError("Isolated portal readiness timed out")
                    time.sleep(.1)
            subprocess.run([str(probe), f"127.0.0.1:{port}"], check=True, timeout=90)
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            print("Stopped the isolated process; existing services were not managed")


if __name__ == "__main__":
    main()
