"""Linux/macOS PTY smoke test using only a test-owned agent and data directory.

Build with cargo build --locked -p rove-cli -p rove-agent --bins first.
Never reads real model settings or contacts a model provider.
"""
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import subprocess
import tempfile
import termios
import time

ROOT = Path(__file__).resolve().parents[1]


def stop(process):
    if process is not None and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


with tempfile.TemporaryDirectory(prefix="rove-tty-") as temp:
    data = Path(temp) / "agent"
    agent = subprocess.Popen(
        [str(ROOT / "target/debug/rove-agent"), "--data-dir", str(data)],
        stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    client = None
    master, slave = pty.openpty()
    transcript = bytearray()
    pending = bytearray()
    try:
        deadline = time.monotonic() + 10
        while not (data / "agent.sock").exists():
            assert agent.poll() is None, "Test agent exited before listening"
            assert time.monotonic() < deadline, "Test agent startup timed out"
            time.sleep(0.01)

        def controlling_terminal():
            os.setsid()
            fcntl.ioctl(slave, termios.TIOCSCTTY, 0)

        client = subprocess.Popen(
            [str(ROOT / "target/debug/rove"), "--data-dir", str(data)],
            stdin=slave, stdout=slave, stderr=slave, preexec_fn=controlling_terminal,
        )

        def expect(text):
            marker = text.encode()
            deadline = time.monotonic() + 10
            while marker not in pending:
                assert time.monotonic() < deadline, f"Prompt timed out: {text}"
                ready, _, _ = select.select([master], [], [], 0.1)
                if ready:
                    chunk = os.read(master, 65536)
                    assert chunk, "CLI closed PTY unexpectedly"
                    pending.extend(chunk)
                    transcript.extend(chunk)
            end = pending.index(marker) + len(marker)
            del pending[:end]

        def send(text):
            os.write(master, (text + "\n").encode())

        expect("选择：")
        send("1")
        expect('"device_id"')
        expect("选择：")
        send("6")
        expect("提供方（默认 openai_compatible）：")
        send("")
        expect("模型接口 URL：")
        send("http://127.0.0.1:1/v1")
        expect("模型名称：")
        send("pty-test")
        expect("API key（隐藏输入，留空表示无需密钥）：")
        secret = "rove-pty-secret-must-not-echo"
        send(secret)
        expect("选择：")
        assert secret.encode() not in transcript, "Interactive key was echoed"
        send("0")
        assert client.wait(timeout=5) == 0
        assert agent.poll() is None, "Closing CLI stopped the background agent"
        result = subprocess.run(
            [str(ROOT / "target/debug/rove"), "--data-dir", str(data), "--json", "model", "show"],
            capture_output=True, check=True, timeout=5,
        )
        value = json.loads(result.stdout)
        assert value["config"]["api_key_configured"] is True
        assert secret.encode() not in result.stdout + result.stderr
        print("PASS: PTY menu, hidden model key, independent agent lifetime")
    finally:
        stop(client)
        os.close(master)
        os.close(slave)
        stop(agent)
