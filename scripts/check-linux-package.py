"""Install/upgrade/remove the development runtime in one disposable container.

No host services or data are mounted. This verifies package layout and data
retention, not systemd PID 1, real GUI, or two-device acceptance.
"""
import argparse
from pathlib import Path
import subprocess
import uuid

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("package", type=Path)
parser.add_argument("--gui", type=Path, help="Also install the Tauri desktop .deb (no display launch)")
args = parser.parse_args()
package = args.package.resolve(strict=True)
assert package.is_file() and package.suffix == ".deb"
script = r"""
set -eu
dpkg --install /tmp/rove-runtime.deb
if test -f /tmp/rove-gui.deb; then
  dpkg --install /tmp/rove-gui.deb
  test -x /usr/bin/rove-gui
  ldd /usr/bin/rove-gui >/tmp/gui-libraries
  if grep -q 'not found' /tmp/gui-libraries; then exit 1; fi
fi
test -x /usr/bin/rove
test -x /usr/lib/rove/rove-agent
/usr/lib/rove/easytier-core --version
systemd-analyze verify /usr/lib/systemd/system/rove-easytier.service
mkdir -m 700 /tmp/rove-package-state
/usr/lib/rove/rove-agent --data-dir /tmp/rove-package-state >/tmp/agent.log 2>&1 &
agent_pid=$!
sleep 1
/usr/bin/rove --data-dir /tmp/rove-package-state --json status >/tmp/before.json
kill -TERM "$agent_pid"
wait "$agent_pid"
dpkg --install /tmp/rove-runtime.deb
/usr/lib/rove/rove-agent --data-dir /tmp/rove-package-state >/tmp/agent.log 2>&1 &
agent_pid=$!
sleep 1
/usr/bin/rove --data-dir /tmp/rove-package-state --json status >/tmp/after.json
python3 -c 'import json; a=json.load(open("/tmp/before.json")); b=json.load(open("/tmp/after.json")); assert a["device_id"] == b["device_id"]'
kill -TERM "$agent_pid"
wait "$agent_pid"
if test -f /tmp/rove-gui.deb; then dpkg --remove rove; fi
dpkg --remove rove-runtime
test -d /tmp/rove-package-state
test ! -e /usr/bin/rove
test ! -e /usr/lib/systemd/system/rove-easytier.service
echo 'PASS: install, same-version upgrade, CLI/agent identity, remove retains data; systemd activation not tested'
"""
command = ["docker", "run", "--rm", "--network", "none", "--name", "rove-package-" + uuid.uuid4().hex[:12],
           "--mount", f"type=bind,source={package},target=/tmp/rove-runtime.deb,readonly"]
if args.gui:
    gui = args.gui.resolve(strict=True)
    assert gui.is_file() and gui.suffix == ".deb"
    command.extend(["--mount", f"type=bind,source={gui},target=/tmp/rove-gui.deb,readonly"])
command.extend(["rove-build-linux:check-20260908", "sh", "-ec", script])
subprocess.run(command, check=True, timeout=120)
