"""Real multi-network TUN tests in disposable Linux containers, no host routing changes."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import uuid
from urllib.parse import urlparse


def run(args, **kwargs):
    return subprocess.check_output(args, text=True, stderr=subprocess.PIPE, timeout=40, **kwargs).strip()


def call(container, op, body=None, network=None, target=None, connection=None):
    command = ["docker", "exec", "-i", container, "/opt/rove", "--data-dir", "/state/rove", "--json"]
    if target:
        command += ["--network", target[0], "--device", target[1]]
    command += ["call", op]
    if network:
        command += ["--path", f"network_id={network}"]
    if connection:
        command += ["--path", f"connection_id={connection}"]
    if body is not None:
        command += ["--body", "-"]
    result = json.loads(run(command, input=json.dumps(body) if body is not None else None))
    return result.get("body", result) if isinstance(result, dict) else result


def wait_state(container, network, state):
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        record = call(container, "get_network", network=network)
        if record["state"] == state:
            return record
        if record["state"] == "failed" and state != "failed":
            raise AssertionError(record)
        time.sleep(.3)
    raise AssertionError(record)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target-dir", required=True, type=Path)
    parser.add_argument("--easytier", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit("Refusing to overwrite an acceptance report")
    containers, checks = [], []
    report = {"result": "FAIL", "scope": "Two isolated Linux containers; not separate hosts or other platforms", "checks": checks}
    with tempfile.TemporaryDirectory(prefix="rove-multinetwork-", dir=args.target_dir.parent) as staging:
        files = [(args.target_dir / "debug/rove-agent", "rove-agent"), (args.target_dir / "debug/rove", "rove"),
                 (args.target_dir / "debug/examples/peer_acceptance_fixture", "fixture"), (args.easytier, "easytier-core")]
        mounts = []
        for source, name in files:
            destination = Path(staging) / name
            shutil.copy2(source, destination)
            mounts += ["--mount", f"type=bind,source={destination},target=/opt/{name},readonly"]
        try:
            for _ in range(2):
                name = "rove-multinetwork-" + uuid.uuid4().hex[:12]
                containers.append(name)
                run(["docker", "run", "-d", "--name", name, "--cpus", "2", "--memory", "1g", "--pids-limit", "512", "--cap-add", "NET_ADMIN",
                     "--device-cgroup-rule", "c 10:200 rwm", "-e", "ROVE_DISPOSABLE_OVERLAY_TEST=1", *mounts,
                     "rove-build-linux:check-20260908", "sh", "-ec",
                     "umask 077; mkdir -p /dev/net /state/easytier; mknod /dev/net/tun c 10 200; "
                     "/opt/easytier-core --daemon --config-dir /state/easytier --rpc-portal 127.0.0.1:15888 "
                     "--rpc-portal-whitelist 127.0.0.1/32 --console-log-level error >/state/easytier.log 2>&1 & "
                     "exec /opt/rove-agent --data-dir /state/rove --easytier-portal 127.0.0.1:15888 --easytier-core /opt/easytier-core"])
                for attempt in range(50):
                    try:
                        call(name, "get_device")
                        break
                    except subprocess.CalledProcessError:
                        if attempt == 49:
                            raise
                        time.sleep(.2)
            a, b = containers
            physical_a = json.loads(run(["docker", "inspect", a]))[0]["NetworkSettings"]["Networks"]["bridge"]["IPAddress"]
            device_b = call(b, "get_device")["device_id"]
            networks = []
            for octet in [10, 20]:
                cfg = {"network_name": f"fixture-{octet}", "network_secret": uuid.uuid4().hex, "dhcp": False,
                       "ipv4_cidr": f"10.241.{octet}.0/24", "bootstrap_peers": []}
                net = call(a, "create_network", {"display_name": f"network-{octet}", "config": cfg, "local_ipv4": f"10.241.{octet}.1"})
                network = net["network_id"]
                call(a, "start_network", network=network)
                record = wait_state(a, network, "running")
                assert record["overlay_cidr"] == cfg["ipv4_cidr"], record
                join = call(a, "get_network_join_config", network=network)
                assert "local_ipv4" not in json.dumps(join) and "listener_url" not in json.dumps(join)
                port = urlparse(record["listener_url"]).port
                assert port and port != 0, record
                join["easytier"]["bootstrap_peers"] = [f"tcp://{physical_a}:{port}"]
                call(b, "import_network", {"source": "manual", "config": join})
                call(b, "update_network", {"display_name": join["display_name"], "easytier": join["easytier"], "local_ipv4": f"10.241.{octet}.2"}, network)
                call(b, "start_network", network=network)
                wait_state(b, network, "running")
                networks.append(network)
            checks.append("two nonoverlapping manual networks run on each device using original EasyTier")
            for network in networks:
                for attempt in range(70):
                    peers = call(a, "list_network_devices", network=network)["items"]
                    if any(peer["device_id"] == device_b and peer["state"] == "online" for peer in peers):
                        break
                    time.sleep(.3)
                assert call(a, "get_device", target=(network, device_b))["device_id"] == device_b
            checks.append("stable remote identity and SDK routing independently on both networks")
            # Synthetic API key stays in agents; no paid provider is contacted.
            api_config = {"provider": "openai_compatible", "base_url": "http://127.0.0.1:1/v1",
                          "api_key": "overlay-sync-fixture-private", "models": [{"model": "test", "name": "overlay API fixture"}], "set_default": False}
            source_catalog = call(a, "import_models", api_config)
            source_connection = source_catalog["connections"][0]["connection_id"]
            sync_body = {"target": {"network_id": networks[1], "device_id": device_b}}
            synced = call(a, "sync_model_connection", sync_body, connection=source_connection)
            assert "overlay-sync-fixture-private" not in json.dumps(synced)
            assert synced["connections"][0]["api_key_configured"]
            assert synced["models"][0]["name"] == "overlay API fixture"
            assert synced["default_model_id"] is None
            assert call(a, "sync_model_connection", sync_body, connection=source_connection) == synced
            changed = {k: api_config[k] for k in ["provider", "base_url", "api_key"]}
            changed["api_key"] = "overlay-sync-fixture-rotated"
            call(a, "update_model_connection", changed, connection=source_connection)
            try:
                call(a, "sync_model_connection", sync_body, connection=source_connection)
                raise AssertionError("Changed API key overwrote target without confirmation")
            except subprocess.CalledProcessError as error:
                assert "model_sync_conflict" in error.stderr
                assert "overlay-sync-fixture" not in error.stderr
            sync_body["overwrite"] = True
            assert call(a, "sync_model_connection", sync_body, connection=source_connection) == synced
            call(a, "delete_model_connection", connection=source_connection)
            assert call(a, "get_model_catalog", target=(networks[1], device_b)) == synced
            checks.append("API key sync over real overlay is private, idempotent, confirmation-gated and independent of source deletion")
            # Test application has its own lifecycle, independent of publication.
            run(["docker", "exec", "-d", b, "/opt/fixture", "app"])
            service = call(b, "publish_service", {"network_id": networks[1], "name": "surviving service", "protocol": "http",
                                                 "target": {"host": "127.0.0.1", "port": 19081}})
            endpoint = service["endpoints"][0]
            assert json.loads(run(["docker", "exec", a, "/opt/fixture", "fetch", endpoint, "authenticated"]))["status"] == 200
            first_listener = call(b, "get_network", network=networks[0])["listener_url"]
            call(b, "stop_network", network=networks[0])
            wait_state(b, networks[0], "stopped")
            assert call(b, "get_network", network=networks[1])["state"] == "running"
            assert call(a, "get_device", target=(networks[1], device_b))["device_id"] == device_b
            assert json.loads(run(["docker", "exec", a, "/opt/fixture", "fetch", endpoint, "authenticated"]))["status"] == 200
            checks.append("stopping one network preserves the other SDK endpoint and authenticated service")
            call(b, "start_network", network=networks[0])
            assert wait_state(b, networks[0], "running")["listener_url"] == first_listener
            checks.append("network restart preserves its local bootstrap listener while another stays active")
            cfg = {"network_name": "conflict", "network_secret": uuid.uuid4().hex, "dhcp": False, "ipv4_cidr": "10.241.0.0/16", "bootstrap_peers": []}
            conflict = call(a, "create_network", {"display_name": "conflict", "config": cfg, "local_ipv4": "10.241.30.1"})["network_id"]
            try:
                call(a, "start_network", network=conflict)
                raise AssertionError("Overlapping network unexpectedly started")
            except subprocess.CalledProcessError as error:
                assert "network_subnet_conflict" in error.stderr, error.stderr
            assert call(a, "get_network", network=conflict)["state"] == "stopped"
            checks.append("containing subnet rejected before creating an overlapping TUN")
            # A separate no-TUN EasyTier peer advertises an already used subnet.
            # It is not another Rove network and never changes the host routes.
            physical_b = json.loads(run(["docker", "inspect", b]))[0]["NetworkSettings"]["Networks"]["bridge"]["IPAddress"]
            secret = uuid.uuid4().hex
            run(["docker", "exec", "-d", b, "/opt/easytier-core", "--no-tun", "--ipv4", "10.241.10.3/24",
                 "--network-name", "automatic-conflict", "--network-secret", secret,
                 "--listeners", "tcp://0.0.0.0:12010", "--console-log-level", "error"])
            cfg = {"network_name": "automatic-conflict", "network_secret": secret, "dhcp": True,
                   "bootstrap_peers": [f"tcp://{physical_b}:12010"]}
            automatic = call(a, "create_network", {"display_name": "automatic-conflict", "config": cfg})["network_id"]
            call(a, "start_network", network=automatic)
            failed = wait_state(a, automatic, "failed")
            assert failed["last_error"]["code"] == "network_subnet_conflict", failed
            assert failed["overlay_addresses"] == []
            for network in networks:
                wait_state(a, network, "running")
            assert json.loads(run(["docker", "exec", a, "/opt/fixture", "fetch", endpoint, "authenticated"]))["status"] == 200
            checks.append("original EasyTier DHCP overlap rejected at runtime without rewriting its address pool")
            report["result"] = "PASS"
        except Exception as error:
            report["error"] = str(error)
            raise
        finally:
            for name in containers:
                subprocess.run(["docker", "rm", "-f", name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=30, check=False)
            with args.output.open("x") as output:
                json.dump(report, output, ensure_ascii=False, indent=2)
            print(json.dumps(report, ensure_ascii=False))


if __name__ == "__main__":
    main()
