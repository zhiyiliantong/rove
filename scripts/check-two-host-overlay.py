"""Explicit two-host Linux acceptance in NEW disposable containers only.

Requires matching prebuilt x86_64/aarch64 agent, CLI and fixture, a verified
EasyTier arm64 archive and a cached arm64 base image on the remote Docker host.
No host network/route/service changes, user data mounts or paid model calls.
The remote base container receives only the host's public CA bundle read-only,
not private keys or user credentials (the runtime package requires ca-certificates).
SSH uses existing keys, or ROVE_TEST_SSH_PASSWORD via sshpass (never persisted).
Remote staging binaries are retained; only test-owned containers are removed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tarfile
import tempfile
import time
import uuid
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", required=True)
    parser.add_argument("--user", default="root")
    parser.add_argument("--port", type=int, default=31010)
    parser.add_argument("--target-dir", required=True, type=Path)
    parser.add_argument("--arm-easytier", required=True, type=Path)
    parser.add_argument("--remote-image", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    import ipaddress
    address = ipaddress.IPv4Address(args.host)
    assert address.is_private and not address.is_loopback, "Use an explicitly authorized private host"
    assert args.user.isalnum() and 1024 <= args.port <= 65535
    assert not args.output.exists(), "Refusing to overwrite acceptance evidence"
    asset = next(a for a in json.loads((ROOT / "packaging/easytier-assets.json").read_text())["assets"] if a["platform"] == "linux" and a["arch"] == "aarch64")
    assert hashlib.sha256(args.arm_easytier.read_bytes()).hexdigest() == asset["sha256"]
    environment = os.environ.copy()
    password = environment.pop("ROVE_TEST_SSH_PASSWORD", None)
    prefix = ["sshpass", "-e"] if password else []
    if password:
        environment["SSHPASS"] = password
    ssh = prefix + ["ssh", "-o", "ConnectTimeout=5", "-o", "StrictHostKeyChecking=yes", f"{args.user}@{args.host}"]

    def execute(command, remote=False, data=None, timeout=45):
        if remote:
            command = ssh + [shlex.join(command)]
        return subprocess.check_output(command, input=data, text=True, timeout=timeout, env=environment).strip()

    class Peer:
        def __init__(self, name, remote):
            self.name, self.remote = name, remote

        def docker(self, *command, data=None, timeout=45):
            return execute(["docker", *command], self.remote, data, timeout)

        def call(self, operation, body=None, path=None, target=None):
            command = ["exec", "-i", self.name, "/opt/rove", "--data-dir", "/state/rove", "--json"]
            if target:
                command += ["--network", target["network_id"], "--device", target["device_id"]]
            command += ["call", operation]
            for key, value in (path or {}).items():
                command += ["--path", f"{key}={value}"]
            if body is not None:
                command += ["--body", "-"]
            result = json.loads(self.docker(*command, data=json.dumps(body) if body is not None else None))
            return result.get("body", result) if isinstance(result, dict) else result

        def fetch(self, url, auth=True):
            return json.loads(self.docker("exec", self.name, "/opt/fixture", "fetch", url, *( ["authenticated"] if auth else [])))

    def until(description, condition, timeout=60):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                if condition():
                    return
            except subprocess.CalledProcessError:
                pass
            time.sleep(.5)
        raise RuntimeError(description)

    stage = execute(["mktemp", "-d", "/var/tmp/rove-acceptance-XXXXXXXX"], remote=True)
    assert stage.startswith("/var/tmp/rove-acceptance-") and all(c.isalnum() or c in "/-_" for c in stage)
    peers = [Peer("rove-acceptance-" + uuid.uuid4().hex[:12], False), Peer("rove-acceptance-" + uuid.uuid4().hex[:12], True)]
    started = []
    checks = []
    report = {"result":"FAIL", "host":str(address), "remote_stage":stage, "checks":checks,
              "scope":"Two separate Linux Docker hosts; not Android/Windows overlay, physical camera or systemd acceptance. Controlled model, no external model credentials."}
    try:
        with tempfile.TemporaryDirectory(prefix="rove-two-host-") as temp:
            temp = Path(temp)
            for peer, directory in zip(peers, [args.target_dir / "debug", args.target_dir / "aarch64-unknown-linux-gnu/debug"]):
                folder = temp / peer.name
                folder.mkdir()
                for name, source in [("rove-agent", directory / "rove-agent"), ("rove", directory / "rove"), ("fixture", directory / "examples/peer_acceptance_fixture")]:
                    shutil.copy2(source, folder / name)
                if peer.remote:
                    with zipfile.ZipFile(args.arm_easytier) as archive:
                        (folder / "easytier-core").write_bytes(archive.read("easytier-linux-aarch64/easytier-core"))
                    (folder / "easytier-core").chmod(0o755)
                    archive_path = temp / "remote-binaries.tar"
                    with tarfile.open(archive_path, "w") as archive:
                        for path in folder.iterdir():
                            archive.add(path, arcname=path.name)
                    subprocess.run(prefix + ["scp", "-o", "StrictHostKeyChecking=yes", str(archive_path), f"{args.user}@{args.host}:{stage}/binaries.tar"], check=True, env=environment, timeout=90)
                    execute(["tar", "xf", stage + "/binaries.tar", "-C", stage], remote=True)
                    source = stage
                else:
                    shutil.copy2(shutil.which("easytier-core"), folder / "easytier-core")
                    source = str(folder)
                command = ["run", "--detach", "--name", peer.name, "--cpus", "2", "--memory", "1g", "--pids-limit", "512", "--cap-add", "NET_ADMIN", "--device-cgroup-rule", "c 10:200 rwm", "--env", "ROVE_DISPOSABLE_OVERLAY_TEST=1"]
                if peer.remote:
                    command += ["--publish", f"{args.host}:{args.port}:11010/tcp"]
                    execute(["test", "-f", "/etc/ssl/certs/ca-certificates.crt"], remote=True)
                    command += ["--mount", "type=bind,source=/etc/ssl/certs/ca-certificates.crt,target=/etc/ssl/certs/ca-certificates.crt,readonly"]
                for name in ["rove-agent", "rove", "fixture", "easytier-core"]:
                    command += ["--mount", f"type=bind,source={source}/{name},target=/opt/{name},readonly"]
                command += [args.remote_image if peer.remote else "rove-build-linux:check-20260908", "sh", "-ec",
                            "umask 077; mkdir -p /dev/net /state/easytier; mknod /dev/net/tun c 10 200; "
                            "/opt/easytier-core --daemon --config-dir /state/easytier --rpc-portal 127.0.0.1:15888 --rpc-portal-whitelist 127.0.0.1/32 --console-log-level error >/state/network.log 2>&1 & "
                            "exec /opt/rove-agent --data-dir /state/rove --easytier-portal 127.0.0.1:15888 --easytier-core /opt/easytier-core"]
                started.append(peer)
                peer.docker(*command)
                until("Agent readiness failed", lambda: peer.call("get_device"))
            a, b = peers
            device_a, device_b = a.call("get_device"), b.call("get_device")
            assert device_a["device_id"] != device_b["device_id"]
            assert device_a["arch"] == "x86_64" and device_b["arch"] == "aarch64"
            report["devices"] = [device_a, device_b]
            network = a.call("create_network", {"display_name":"two-host acceptance", "bootstrap_peers":[f"tcp://{args.host}:{args.port}"]})
            path = {"network_id":network["network_id"]}
            join = a.call("get_network_join_config", path=path)
            assert b.call("import_network", {"source":"manual", "config":join})["created"]
            for peer in peers:
                peer.call("start_network", path=path)
            target = {"network_id":network["network_id"], "device_id":device_b["device_id"]}
            def online():
                return all(p.call("get_network", path=path)["state"] == "running" for p in peers) and any(p["device_id"] == target["device_id"] and p["state"] == "online" for p in a.call("list_network_devices", path=path)["items"])
            until("Cross-host DHCP/discovery failed", online)
            assert a.call("get_storage_usage", target=target)["device_id"] == device_b["device_id"]
            checks += ["cross-host DHCP/TUN", "portable share/import", "stable identity discovery", "SDK remote target"]
            b.docker("exec", "--detach", b.name, "sh", "-c", "/opt/fixture model " + shlex.quote(network["network_id"]) + " >/state/model.log 2>&1")
            until("Fixture model did not start", lambda: b.fetch("http://127.0.0.1:19080/v1/chat/completions", False)["status"] != 0, timeout=15)
            a.call("set_model_config", {"provider":"openai_compatible", "base_url":"http://127.0.0.1:19080/v1", "model":"fixture", "api_key":None}, target=target)
            session = a.call("create_session", {"title":"two-host local ownership", "execution_target":target})
            assert session["device_id"] == device_a["device_id"]
            job = a.call("submit_run", {"request_id":str(uuid.uuid4()), "message":"Deploy and publish the isolated fixture service"}, path={"session_id":session["session_id"]})
            events = a.docker("exec", a.name, "/opt/rove", "--data-dir", "/state/rove", "--json", "run", "watch", job["run_id"])
            # These are isolated fixture-only messages, not a user's session.
            report["fixture_events"] = events[-65536:]
            report["fixture_snapshot"] = a.call("get_run", path={"run_id":job["run_id"]})
            report["fixture_model_log"] = b.docker("exec", b.name, "head", "-c", "8192", "/state/model.log")
            assert "Controlled two-host deployment finished" in events
            assert a.call("get_run", path={"run_id":job["run_id"]})["run"]["status"] == "succeeded"
            listing = json.loads(a.docker("exec", a.name, "/opt/rove", "--data-dir", "/state/rove", "--network", target["network_id"], "--device", target["device_id"], "--json", "service", "list", "--network-id", target["network_id"]))
            service = listing.get("body", listing)["items"][0]
            url = service["endpoints"][0]
            assert a.fetch(url) == {"status":200,"body":"rove-two-host-service"}
            assert a.fetch(url, False)["status"] == 401
            checks += ["remote model configuration", "local-owned remote job", "Rig tool deployment", "local socket remote SSE", "authenticated service access"]
            b.call("stop_network", path=path)
            until("Network stop failed", lambda:b.call("get_network", path=path)["state"] == "stopped")
            assert a.fetch(url)["status"] == 0
            assert b.fetch("http://127.0.0.1:19081")["status"] == 200
            b.call("start_network", path=path)
            until("Network recovery failed", online)
            restored = a.call("get_service", path={"service_id":service["service_id"]}, target=target)
            assert restored["listen_port"] == service["listen_port"]
            assert a.fetch(restored["endpoints"][0])["status"] == 200
            a.call("unpublish_service", path={"service_id":service["service_id"]}, target=target)
            assert a.fetch(restored["endpoints"][0])["status"] == 0
            assert b.fetch("http://127.0.0.1:19081")["status"] == 200
            checks += ["network outage closes entry", "target survives network stop", "same-port recovery", "unpublish preserves application"]
            report["result"] = "PASS"
    finally:
        for peer in started:
            try:
                peer.docker("rm", "--force", peer.name, timeout=20)
            except (subprocess.SubprocessError, OSError):
                report.setdefault("cleanup_required", []).append(peer.name)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open("x") as output:
            json.dump(report, output, indent=2)
        print(json.dumps(report, indent=2))
        print("Test containers removed" if not report.get("cleanup_required") else "Check cleanup_required for remaining test containers")
        print("Remote staging binaries retained at " + stage)


if __name__ == "__main__":
    main()
