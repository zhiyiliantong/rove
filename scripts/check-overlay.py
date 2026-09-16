"""Real EasyTier/TUN integration in two disposable containers, not two machines.

Uses cached image and five read-only binaries; no host network, route, service,
credential directory or application data is mounted. Always removes own containers.
Build agent with --features easytier and CLI before running this script.
"""
import json
from pathlib import Path
import shutil
import shlex
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
IMAGE = "rove-build-linux:check-20260908"


def run(args, **kwargs):
    return subprocess.check_output(args, text=True, timeout=40, **kwargs).strip()


def call(container, operation, body=None, path=None, target=None):
    command = ["docker", "exec", "-i", container, "/opt/rove", "--data-dir", "/state/rove", "--json"]
    if target:
        command.extend(["--network", target[0], "--device", target[1]])
    command.extend(["call", operation])
    if path:
        for key, value in path.items():
            command.extend(["--path", f"{key}={value}"])
    if body is not None:
        command.extend(["--body", "-"])
    value = json.loads(run(command, input=json.dumps(body) if body is not None else None))
    return value.get("body", value) if isinstance(value, dict) else value


def main():
    binaries = [(ROOT / "target/debug/rove-agent", "/opt/rove-agent"),
                (ROOT / "target/debug/examples/easytier_status_probe", "/opt/status-probe"),
                (ROOT / "target/debug/examples/overlay_address_probe", "/opt/address-probe"),
                (ROOT / "target/debug/rove", "/opt/rove"),
                (Path(shutil.which("easytier-core") or "/missing"), "/opt/easytier-core")]
    for binary, _ in binaries:
        assert binary.is_file(), f"Missing binary: {binary}"
    # Cargo may replace target/debug binaries during another test/build. Keep
    # immutable per-run copies, including across container restart.
    artifacts = tempfile.TemporaryDirectory(prefix="rove-overlay-binaries-")
    frozen = []
    for binary, destination in binaries:
        snapshot = Path(artifacts.name) / Path(destination).name
        shutil.copy2(binary, snapshot)
        frozen.append((snapshot, destination))
    binaries = frozen
    containers = []
    try:
        for suffix in ["a", "b"]:
            name = "rove-overlay-" + uuid.uuid4().hex[:12] + "-" + suffix
            command = ["docker", "run", "--detach", "--name", name, "--cap-add", "NET_ADMIN",
                       "--device-cgroup-rule", "c 10:200 rwm"]
            for binary, destination in binaries:
                command.extend(["--mount", f"type=bind,source={binary.resolve()},target={destination},readonly"])
            command.extend([IMAGE, "sh", "-ec",
                "umask 077; mkdir -p /dev/net /state/easytier; mknod /dev/net/tun c 10 200; "
                "/opt/easytier-core --daemon --config-dir /state/easytier --rpc-portal 127.0.0.1:15888 "
                "--rpc-portal-whitelist 127.0.0.1/32 --console-log-level error >/dev/null 2>&1 & "
                "exec /opt/rove-agent --data-dir /state/rove --easytier-portal 127.0.0.1:15888 "
                "--easytier-core /opt/easytier-core"])
            containers.append(name)
            run(command)
            deadline = time.monotonic() + 15
            while True:
                try:
                    call(name, "get_device")
                    break
                except subprocess.CalledProcessError:
                    state = json.loads(run(["docker", "inspect", name]))[0]["State"]
                    if not state["Running"]:
                        raise RuntimeError(f"Test agent exited during startup: exit code {state['ExitCode']}")
                    if time.monotonic() >= deadline:
                        raise RuntimeError("Container agent did not become ready")
                    time.sleep(.2)
        a, b = containers
        details = json.loads(run(["docker", "inspect", a]))[0]
        physical_a = details["NetworkSettings"]["Networks"]["bridge"]["IPAddress"]
        network = call(a, "create_network", {"display_name": "isolated overlay test", "bootstrap_peers": [f"tcp://{physical_a}:11010"]})
        # CLI output uses the shared Response envelope.
        if "body" in network:
            network = network["body"]
        network_id = network["network_id"]
        path = {"network_id": network_id}
        config = call(a, "get_network_join_config", path=path)
        if "body" in config:
            config = config["body"]
        call(b, "import_network", {"source": "manual", "config": config})
        call(a, "start_network", path=path)
        call(b, "start_network", path=path)
        deadline = time.monotonic() + 45
        states = []
        while time.monotonic() < deadline:
            states = [call(node, "get_network", path=path) for node in containers]
            states = [state.get("body", state) for state in states]
            if all(state["state"] == "running" for state in states):
                break
            time.sleep(.5)
        else:
            for node in containers:
                print(run(["docker", "exec", node, "/opt/status-probe", "127.0.0.1:15888"]))
                print(run(["docker", "exec", node, "sh", "-c", "ls /sys/class/net; for f in /sys/class/net/*/tun_flags; do test ! -f \"$f\" || head -c 64 \"$f\"; done"]))
            raise RuntimeError("Overlay not ready: " + json.dumps(states))
        assert states[0]["overlay_addresses"] != states[1]["overlay_addresses"]
        ip_b = states[1]["overlay_addresses"][0]
        # Python sends from A's network namespace through EasyTier to B's HTTP.
        hello = json.loads(run(["docker", "exec", a, "python3", "-c",
            "import urllib.request; r=urllib.request.Request(" + repr(f"http://{ip_b}:43190/v1/hello") +
            ",headers={'X-Rove-Network-Id':" + repr(network_id) + "}); "
            "print(urllib.request.build_opener(urllib.request.ProxyHandler({})).open(r,timeout=5).read().decode())"]))
        assert hello["network_id"] == network_id
        device_b = hello["device_id"]
        target = (network_id, device_b)
        deadline = time.monotonic() + 15
        while True:
            peers = call(a, "list_network_devices", path=path)["items"]
            if any(peer["device_id"] == device_b and peer["state"] == "online" for peer in peers):
                break
            if time.monotonic() >= deadline:
                raise RuntimeError("Stable device discovery did not complete")
            time.sleep(.2)
        remote = call(a, "get_device", target=target)
        assert remote["device_id"] == device_b
        app_code = """from http.server import BaseHTTPRequestHandler,HTTPServer
class App(BaseHTTPRequestHandler):
 def do_GET(self):
  allowed=self.headers.get('Authorization')=='Bearer rove-test-only'
  self.send_response(200 if allowed else 401); self.end_headers(); self.wfile.write(b'rove-overlay-service' if allowed else b'unauthorized')
 def log_message(self,*args): pass
HTTPServer(('127.0.0.1',19081),App).serve_forever()
"""
        command = "python3 -c " + shlex.quote(app_code) + " >/dev/null 2>&1 &"
        tools = [
            {"index": 0, "id": "start_app", "type": "function", "function": {"name": "system_exec", "arguments": json.dumps({"command": command})}},
            {"index": 1, "id": "publish_app", "type": "function", "function": {"name": "publish_service", "arguments": json.dumps({"network_id": network_id, "name": "overlay test service", "target": {"host": "127.0.0.1", "port": 19081}, "protocol": "http", "access_info": "Bearer rove-test-only"})}},
        ]
        provider = """import json
from http.server import BaseHTTPRequestHandler,HTTPServer
TOOLS=""" + repr(tools) + """
class Model(BaseHTTPRequestHandler):
 def do_POST(self):
  body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
  finished=any(m.get('role')=='tool' for m in body['messages'])
  delta={'role':'assistant','content':'overlay tools finished'} if finished else {'role':'assistant','tool_calls':TOOLS}
  chunks=[{'id':'test','object':'chat.completion.chunk','created':1,'model':'test','choices':[{'index':0,'delta':delta,'finish_reason':None}]},{'id':'test','object':'chat.completion.chunk','created':1,'model':'test','choices':[{'index':0,'delta':{},'finish_reason':'stop' if finished else 'tool_calls'}]}]
  self.send_response(200); self.send_header('Content-Type','text/event-stream'); self.end_headers()
  for chunk in chunks: self.wfile.write(('data: '+json.dumps(chunk)+'\\n\\n').encode())
  self.wfile.write(b'data: [DONE]\\n\\n')
 def log_message(self,*args): pass
HTTPServer(('127.0.0.1',19080),Model).serve_forever()
"""
        run(["docker", "exec", "--detach", b, "python3", "-c", provider])
        call(a, "set_model_config", {"provider": "openai_compatible", "base_url": "http://127.0.0.1:19080/v1", "model": "test", "api_key": None}, target=target)
        session = call(a, "create_session", {"title": "real overlay controlled model"}, target=target)
        submitted = call(a, "submit_run", {"request_id": str(uuid.uuid4()), "message": "start and publish test service"}, path={"session_id": session["session_id"]}, target=target)
        run_id = submitted["run_id"]
        events = run(["docker", "exec", a, "/opt/rove", "--data-dir", "/state/rove", "--network", network_id, "--device", device_b, "--json", "run", "watch", run_id])
        assert "overlay tools finished" in events
        snapshot = call(a, "get_run", path={"run_id": run_id}, target=target)
        assert snapshot.get("run", snapshot)["status"] == "succeeded", snapshot
        service_list = run(["docker", "exec", a, "/opt/rove", "--data-dir", "/state/rove", "--network", network_id, "--device", device_b, "--json", "service", "list", "--network-id", network_id])
        services = json.loads(service_list)
        services = services.get("body", services)["items"]
        assert len(services) == 1 and services[0]["state"] == "published", services
        service = services[0]
        fetch = "import urllib.request,urllib.error; o=urllib.request.build_opener(urllib.request.ProxyHandler({})); "
        response = run(["docker", "exec", a, "python3", "-c", fetch + "r=urllib.request.Request(" + repr(service["endpoints"][0]) + ",headers={'Authorization':'Bearer rove-test-only'}); print(o.open(r,timeout=5).read().decode())"])
        assert response == "rove-overlay-service"
        denied = fetch + "\ntry:\n o.open(" + repr(service["endpoints"][0]) + ",timeout=5); raise AssertionError('Missing application authentication was accepted')\nexcept urllib.error.HTTPError as error:\n assert error.code==401\n"
        run(["docker", "exec", a, "python3", "-c", denied])
        physical_b = json.loads(run(["docker", "inspect", b]))[0]["NetworkSettings"]["Networks"]["bridge"]["IPAddress"]
        route_probe = """import socket,struct,urllib.request
DEST=""" + repr(ip_b) + "\nGATEWAY=" + repr(physical_b) + "\nNETWORK=" + repr(network_id) + """
def attr(kind,data):
 value=struct.pack('HH',len(data)+4,kind)+data
 return value+b'\\0'*((-len(value))%4)
def route(kind):
 payload=struct.pack('BBBBBBBBI',socket.AF_INET,32,0,0,254,4,0,1,0)
 payload+=attr(1,socket.inet_aton(DEST))+attr(5,socket.inet_aton(GATEWAY))+attr(4,struct.pack('I',socket.if_nametoindex('eth0')))
 flags=1|4|(0x400|0x200 if kind==24 else 0)
 message=struct.pack('IHHII',len(payload)+16,kind,flags,1,0)+payload
 with socket.socket(socket.AF_NETLINK,socket.SOCK_RAW,0) as control:
  control.settimeout(3); control.bind((0,0)); control.send(message); reply=control.recv(4096)
  assert struct.unpack_from('i',reply,16)[0]==0,'Route operation failed'
for port in [15888,43190]:
 try:
  connection=socket.create_connection((GATEWAY,port),timeout=1); connection.close()
 except OSError: pass
 else: raise AssertionError('Physical management/API port exposed')
route(24)
try:
 request=urllib.request.Request('http://'+DEST+':43190/v1/hello',headers={'X-Rove-Network-Id':NETWORK})
 try: urllib.request.build_opener(urllib.request.ProxyHandler({})).open(request,timeout=2)
 except OSError: pass
 else: raise AssertionError('Overlay API reachable through physical ingress')
finally: route(25)
"""
        run(["docker", "exec", a, "python3", "-c", route_probe])
        # Restart only our disposable B container; its writable layer preserves
        # the agent DB and EasyTier config, then is removed in finally.
        run(["docker", "restart", "--time", "3", b])
        deadline = time.monotonic() + 45
        while True:
            try:
                record = call(b, "get_network", path=path)
                peers = call(a, "list_network_devices", path=path)["items"]
                if record["state"] == "running" and any(p["device_id"] == device_b and p["state"] == "online" for p in peers):
                    break
            except subprocess.CalledProcessError:
                pass
            if time.monotonic() >= deadline:
                raise RuntimeError("Restart did not restore the network/discovered identity")
            time.sleep(.5)
        assert call(b, "get_device")["device_id"] == device_b
        restored = call(a, "get_service", path={"service_id":service["service_id"]}, target=target)
        assert restored["state"] == "published" and restored["listen_port"] == service["listen_port"]
        # The test app is not an installed OS service. Bring it back explicitly;
        # Rove restores its proxy, not arbitrary application processes.
        run(["docker", "exec", "--detach", b, "python3", "-c", app_code])
        time.sleep(.2)
        response = run(["docker", "exec", a, "python3", "-c", fetch + "r=urllib.request.Request(" + repr(restored["endpoints"][0]) + ",headers={'Authorization':'Bearer rove-test-only'}); print(o.open(r,timeout=5).read().decode())"])
        assert response == "rove-overlay-service"
        # Explicit test-only upstream fault injection. Product configuration
        # remains DHCP; only this disposable instance is given a new address.
        previous_ip = record["overlay_addresses"][0]
        new_ip = previous_ip.rsplit(".", 1)[0] + ".240"
        assert new_ip not in [previous_ip, states[0]["overlay_addresses"][0]]
        run(["docker", "exec", "--env", "ROVE_DISPOSABLE_OVERLAY_TEST=1", b, "/opt/address-probe", new_ip])
        deadline = time.monotonic() + 45
        while True:
            record = call(b, "get_network", path=path)
            peers = call(a, "list_network_devices", path=path)["items"]
            if record["state"] == "running" and record["overlay_addresses"] == [new_ip] and any(
                    p["device_id"] == device_b and p["state"] == "online" and new_ip in p["overlay_addresses"] for p in peers):
                break
            if time.monotonic() >= deadline:
                raise RuntimeError("Changed address did not refresh the listener/discovery")
            time.sleep(.5)
        changed = call(a, "get_service", path={"service_id": service["service_id"]}, target=target)
        assert changed["listen_port"] == service["listen_port"] and new_ip in changed["endpoints"][0]
        response = run(["docker", "exec", a, "python3", "-c", fetch + "r=urllib.request.Request(" + repr(changed["endpoints"][0]) + ",headers={'Authorization':'Bearer rove-test-only'}); print(o.open(r,timeout=5).read().decode())"])
        assert response == "rove-overlay-service"
        old_entry = "import socket\nfor port in [43190," + str(service["listen_port"]) + "]:\n try:\n  s=socket.create_connection((" + repr(previous_ip) + ",port),timeout=1); s.close()\n except OSError: pass\n else: raise AssertionError('Old overlay entry still accepts connections')\n"
        run(["docker", "exec", a, "python3", "-c", old_entry])
        spare = call(b, "create_network", {"display_name": "preserved offline config", "bootstrap_peers": []})
        # Fail only B's dedicated test network process, not the agent or app.
        run(["docker", "exec", b, "python3", "-c", "import pathlib,os,signal\nfor p in pathlib.Path('/proc').iterdir():\n if p.name.isdigit():\n  try: name=(p/'comm').read_text().strip()\n  except OSError: continue\n  if name=='easytier-core': os.kill(int(p.name),signal.SIGTERM)\n"])
        deadline = time.monotonic() + 25
        while True:
            failed = call(b, "get_network", path=path)
            if failed["state"] == "failed" and failed["last_error"]:
                break
            if time.monotonic() >= deadline:
                raise RuntimeError("Network process failure was not reported")
            time.sleep(.5)
        assert call(b, "get_network", path={"network_id": spare["network_id"]})["state"] == "stopped"
        assert len(call(b, "list_networks")["items"]) == 2
        run(["docker", "exec", "--detach", b, "/opt/easytier-core", "--daemon", "--config-dir", "/state/easytier",
             "--rpc-portal", "127.0.0.1:15888", "--rpc-portal-whitelist", "127.0.0.1/32", "--console-log-level", "error"])
        deadline = time.monotonic() + 45
        while True:
            record = call(b, "get_network", path=path)
            peers = call(a, "list_network_devices", path=path)["items"]
            if record["state"] == "running" and any(p["device_id"] == device_b and p["state"] == "online" for p in peers):
                break
            if time.monotonic() >= deadline:
                raise RuntimeError("Dedicated network process did not recover")
            time.sleep(.5)
        call(a, "unpublish_service", path={"service_id": service["service_id"]}, target=target)
        local_response = run(["docker", "exec", b, "python3", "-c", fetch + "r=urllib.request.Request('http://127.0.0.1:19081',headers={'Authorization':'Bearer rove-test-only'}); print(o.open(r,timeout=5).read().decode())"])
        assert local_response == "rove-overlay-service"
        for node in containers:
            call(node, "stop_network", path=path)
        deadline = time.monotonic() + 20
        while any(call(node, "get_network", path=path)["state"] != "stopped" for node in containers):
            if time.monotonic() >= deadline:
                raise RuntimeError("Network stop was not confirmed")
            time.sleep(.2)
        print("PASS: DHCP/TUN, discovery, CLI forwarding, Rig controlled-provider tools, remote SSE, authenticated proxy, physical rejection, restart/address refresh and confirmed stop. Two containers, not physical-device/platform acceptance.")
    finally:
        for name in containers:
            subprocess.run(["docker", "rm", "--force", name], check=False, stdout=subprocess.DEVNULL, timeout=15)
        if containers:
            print("Removed test-owned containers and their temporary state; no user data was mounted.")
        artifacts.cleanup()


if __name__ == "__main__":
    main()
