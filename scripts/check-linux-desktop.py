"""Actual GTK/WebKit + systemd user-service acceptance, with isolated Rove data.

Requires systemd user manager, Xvfb, ImageMagick, xdotool, DBus and system Python
GI/AT-SPI. Does not install packages, enable boot services or join host networks.
The model is a loopback deterministic SSE fixture, not a paid model. Output data
is retained for inspection. Run with /usr/bin/python3, not the project's venv.
"""
import argparse
import ctypes
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import threading
import time
import uuid


def run(*args, **kwargs):
    return subprocess.check_output(args, text=True, timeout=30, **kwargs).strip()


def until(check, seconds=30):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        result = check()
        if result:
            return result
        time.sleep(0.2)
    raise TimeoutError("Acceptance condition did not become true")


def close_window(window):
    # Send the same WM_DELETE_WINDOW event as a window manager close button.
    # xdotool windowclose destroys the X window, which is not a graceful close.
    class ClientMessage(ctypes.Structure):
        _fields_ = [("type", ctypes.c_int), ("serial", ctypes.c_ulong),
                    ("send_event", ctypes.c_int), ("display", ctypes.c_void_p),
                    ("window", ctypes.c_ulong), ("message_type", ctypes.c_ulong),
                    ("format", ctypes.c_int), ("data", ctypes.c_long * 5)]

    class Event(ctypes.Union):
        _fields_ = [("client", ClientMessage), ("pad", ctypes.c_long * 24)]

    x11 = ctypes.CDLL("libX11.so.6")
    x11.XOpenDisplay.argtypes = [ctypes.c_char_p]
    x11.XOpenDisplay.restype = ctypes.c_void_p
    x11.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
    x11.XInternAtom.restype = ctypes.c_ulong
    x11.XSendEvent.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int,
                             ctypes.c_long, ctypes.POINTER(Event)]
    x11.XFlush.argtypes = [ctypes.c_void_p]
    x11.XCloseDisplay.argtypes = [ctypes.c_void_p]
    display = x11.XOpenDisplay(None)
    assert display, "Missing X display"
    try:
        event = Event()
        event.client.type = 33
        event.client.display = display
        event.client.window = window
        event.client.message_type = x11.XInternAtom(display, b"WM_PROTOCOLS", 0)
        event.client.format = 32
        event.client.data[0] = x11.XInternAtom(display, b"WM_DELETE_WINDOW", 0)
        assert x11.XSendEvent(display, window, 0, 0, ctypes.byref(event))
        x11.XFlush(display)
    finally:
        x11.XCloseDisplay(display)


def inspect_gui(args):
    # This branch runs on a fresh DBus session; accessibility settings do not
    # alter the user's desktop preferences or inspect unrelated applications.
    import gi
    gi.require_version("Atspi", "2.0")
    from gi.repository import Atspi
    for name in ["IsEnabled", "ScreenReaderEnabled"]:
        run("gdbus", "call", "--session", "--dest", "org.a11y.Bus",
            "--object-path", "/org/a11y/bus", "--method",
            "org.freedesktop.DBus.Properties.Set", "org.a11y.Status", name, "<true>")
    env = dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1", WEBKIT_DISABLE_COMPOSITING_MODE="1",
               GTK_MODULES="atk-bridge", ROVE_DATA_DIR=str(args.output / "data"),
               XDG_CACHE_HOME=str(args.output / "cache"), LANG="en_US.UTF-8", LC_ALL="en_US.UTF-8")
    log = (args.output / "gui.log").open("a")
    gui = subprocess.Popen([str(args.gui)], env=env, stdout=log, stderr=log)

    def nodes(node=None, depth=0):
        if depth > 20:
            return
        node = Atspi.get_desktop(0) if node is None else node
        yield node
        for i in range(node.get_child_count()):
            yield from nodes(node.get_child_at_index(i), depth + 1)

    def content():
        assert gui.poll() is None, "GUI exited before rendering; see gui.log"
        texts = []
        for node in nodes():
            texts.append(node.get_name() or "")
            if node.get_text_iface():
                texts.append(Atspi.Text.get_text(node, 0, -1))
        return "\n".join(texts)

    def click(name):
        for node in nodes():
            if node.get_name() == name and node.get_action_iface() and node.get_n_actions():
                assert node.do_action(0)
                return True
        return False

    try:
        until(lambda: "Agent connected" in content())
        until(lambda: "desktop-lifecycle-acceptance" in content())
        if args.expect:
            until(lambda: args.expect in content())
        window = int(run("xdotool", "search", "--onlyvisible", "--pid", str(gui.pid),
                         "--name", "^Rove$").splitlines()[0])
        run("import", "-window", str(window), str(args.output / (args.phase + "-conversation.png")))
        until(lambda: click("Settings"))
        until(lambda: click("Devices and execution targets"))
        until(lambda: args.device_id in content())
        run("import", "-window", str(window), str(args.output / (args.phase + ".png")))
        close_window(window)
        assert gui.wait(timeout=15) == 0, "Window close was not graceful"
    except Exception:
        (args.output / (args.phase + "-failure.txt")).write_text(content())
        subprocess.run(["import", "-window", "root", str(args.output / (args.phase + "-failure.png"))], timeout=10)
        raise
    finally:
        if gui.poll() is None:
            gui.terminate()
            gui.wait(timeout=15)
        log.close()


def main(args):
    args.output.mkdir(parents=True, exist_ok=False)
    unit = "rove-desktop-check-" + uuid.uuid4().hex[:10]
    network_unit = unit + "-network"
    agent_args = []
    complete = threading.Event()
    requested = threading.Event()

    class Model(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            self.rfile.read(int(self.headers["Content-Length"]))
            requested.set()
            if not complete.wait(90):
                self.send_error(504)
                return
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            for delta, finish in [({"role": "assistant", "content": "Desktop job survived GUI close."}, None), ({}, "stop")]:
                chunk = {"id": "desktop-fixture", "object": "chat.completion.chunk", "created": 1,
                         "model": "fixture", "choices": [{"index": 0, "delta": delta, "finish_reason": finish}]}
                self.wfile.write(("data: " + json.dumps(chunk) + "\n\n").encode())
            self.wfile.write(b"data: [DONE]\n\n")

    model = ThreadingHTTPServer(("127.0.0.1", 0), Model)
    threading.Thread(target=model.serve_forever, daemon=True).start()
    read_fd, write_fd = os.pipe()
    xvfb = subprocess.Popen(["Xvfb", "-displayfd", str(write_fd), "-screen", "0", "1280x900x24",
                             "-nolisten", "tcp"], pass_fds=(write_fd,))
    os.close(write_fd)
    with os.fdopen(read_fd) as pipe:
        display = ":" + pipe.readline().strip()

    def cli(*words, body=None):
        return json.loads(run(str(args.cli), "--data-dir", str(args.output / "data"),
                              "--json", *words, input=json.dumps(body) if body is not None else None))

    def ready():
        try:
            return cli("status")
        except subprocess.CalledProcessError:
            return None

    try:
        if args.easytier_core:
            assert os.geteuid() == 0, "Network service acceptance needs an authorized system service operator"
            (args.output / "network").mkdir(mode=0o700)
            with socket.socket() as reservation:
                reservation.bind(("127.0.0.1", 0))
                portal = reservation.getsockname()[1]
            run("systemd-run", "--unit", network_unit, "--collect", "--property=UMask=0077",
                "--property=NoNewPrivileges=true", "--property=ProtectHome=true",
                "--property=ProtectSystem=strict", "--property=PrivateTmp=true",
                "--property=CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW",
                "--property=DevicePolicy=closed", "--property=DeviceAllow=/dev/net/tun rw",
                "--property=ReadWritePaths=" + str(args.output), str(args.easytier_core),
                "--daemon", "--config-dir", str(args.output / "network"),
                "--rpc-portal", f"127.0.0.1:{portal}", "--rpc-portal-whitelist", "127.0.0.1/32")

            def portal_ready():
                try:
                    with socket.create_connection(("127.0.0.1", portal), timeout=0.5):
                        return True
                except OSError:
                    return False

            until(portal_ready)
            agent_args = ["--easytier-portal", f"127.0.0.1:{portal}", "--easytier-core", str(args.easytier_core)]
        run("systemd-run", "--user", "--unit", unit, "--collect", "--property=UMask=0077",
            str(args.agent), "--data-dir", str(args.output / "data"), *agent_args)
        before = until(ready)
        pid = run("systemctl", "--user", "show", unit, "--property=MainPID", "--value")
        cli("model", "set", body={"provider": "openai_compatible", "base_url": f"http://127.0.0.1:{model.server_port}/v1",
                                  "api_key": "local-fixture-not-a-secret", "model": "fixture"})
        session = cli("session", "create", "--title", "desktop-lifecycle-acceptance")
        job = cli("session", "send", session["session_id"], "Check window lifecycle using a deterministic fixture.")
        assert requested.wait(15), "Model was not called"
        for phase, expected in [("before-close", ""), ("after-reopen", "Desktop job survived GUI close.")]:
            subprocess.run(["dbus-run-session", "--", sys.executable, __file__, "--inspect",
                            "--gui", str(args.gui), "--cli", str(args.cli), "--agent", str(args.agent),
                            "--output", str(args.output), "--device-id", before["device_id"],
                            "--phase", phase, "--expect", expected],
                           env=dict(os.environ, DISPLAY=display, GSETTINGS_BACKEND="memory",
                                    XDG_CONFIG_HOME=str(args.output / "config")), check=True, timeout=90)
            assert run("systemctl", "--user", "is-active", unit) == "active"
            assert run("systemctl", "--user", "show", unit, "--property=MainPID", "--value") == pid
            if phase == "before-close":
                assert cli("run", "show", job["run_id"])["run"]["status"] == "running"
                complete.set()
                until(lambda: cli("run", "show", job["run_id"])["run"]["status"] == "succeeded")
        cli("model", "clear")
        run("systemctl", "--user", "restart", unit)
        after = until(ready)
        assert after["device_id"] == before["device_id"]
        assert cli("run", "show", job["run_id"])["run"]["status"] == "succeeded"
        report = {"status": "PASS", "device_id": before["device_id"], "run_id": job["run_id"],
                  "checks": ["native WebKit displays CLI session and matching device identity",
                             "graceful window close preserves active systemd agent and running job",
                             "job finishes after GUI exit; reopened native GUI reads its output",
                             "systemd agent restart preserves identity and finished history"],
                  "scope": "Actual Xvfb GTK/WebKit, transient user service, loopback model fixture; no host overlay changes"}
        if args.easytier_core:
            assert run("systemctl", "is-active", network_unit) == "active"
            report["checks"].append("dedicated hardened system network service and user agent portal integration")
        (args.output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report, indent=2))
    finally:
        complete.set()
        subprocess.run(["systemctl", "--user", "stop", unit], check=False, timeout=20)
        if args.easytier_core:
            subprocess.run(["systemctl", "stop", network_unit], check=False, timeout=20)
        model.shutdown()
        xvfb.terminate()
        xvfb.wait(timeout=10)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ["gui", "cli", "agent", "output"]:
        parser.add_argument("--" + field, type=Path, required=True)
    parser.add_argument("--inspect", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--easytier-core", type=Path, help="Also test a transient system network unit; requires root")
    parser.add_argument("--device-id", default="")
    parser.add_argument("--phase", default="")
    parser.add_argument("--expect", default="")
    options = parser.parse_args()
    for field in ["gui", "cli", "agent", "output"]:
        setattr(options, field, getattr(options, field).resolve())
    inspect_gui(options) if options.inspect else main(options)
