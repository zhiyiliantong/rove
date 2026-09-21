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
wait_agent() {
  attempts=0
  until /usr/bin/rove --data-dir /tmp/rove-package-state --json status >"$1" 2>/tmp/agent-probe.log; do
    attempts=$((attempts + 1))
    if ! kill -0 "$agent_pid" 2>/dev/null || test "$attempts" -ge 50; then
      cat /tmp/agent.log /tmp/agent-probe.log >&2
      return 1
    fi
    sleep 0.2
  done
}
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
wait_agent /tmp/before.json
/usr/bin/rove --data-dir /tmp/rove-package-state --json session create --title package-archive >/tmp/session.json
session_id=$(python3 -c 'import json; print(json.load(open("/tmp/session.json"))["session_id"])')
/usr/bin/rove --data-dir /tmp/rove-package-state --json session archive "$session_id"
kill -TERM "$agent_pid"
wait "$agent_pid"
dpkg --install /tmp/rove-runtime.deb
/usr/lib/rove/rove-agent --data-dir /tmp/rove-package-state >/tmp/agent.log 2>&1 &
agent_pid=$!
wait_agent /tmp/after.json
python3 -c 'import json; a=json.load(open("/tmp/before.json")); b=json.load(open("/tmp/after.json")); assert a["device_id"] == b["device_id"]'
/usr/bin/rove --data-dir /tmp/rove-package-state --json session list --archived true >/tmp/archives.json
python3 -c 'import json; a=json.load(open("/tmp/session.json")); b=json.load(open("/tmp/archives.json")); assert b["items"][0]["session_id"] == a["session_id"]; assert b["items"][0]["archived_at"]'
/usr/bin/rove --data-dir /tmp/rove-package-state --json session restore "$session_id"
/usr/bin/rove --data-dir /tmp/rove-package-state --json session archive "$session_id"
/usr/bin/rove --data-dir /tmp/rove-package-state --json session delete "$session_id" --yes
kill -TERM "$agent_pid"
wait "$agent_pid"
if test -f /tmp/rove-gui.deb; then dpkg --remove rove; fi
dpkg --remove rove-runtime
test -d /tmp/rove-package-state
test ! -e /usr/bin/rove
test ! -e /usr/lib/systemd/system/rove-easytier.service
echo 'PASS: install, same-version upgrade, identity and archive persistence, restore/delete via CLI, remove retains data; systemd activation not tested'
"""
command = ["docker", "run", "--rm", "--network", "none", "--name", "rove-package-" + uuid.uuid4().hex[:12],
           "--mount", f"type=bind,source={package},target=/tmp/rove-runtime.deb,readonly"]
if args.gui:
    gui = args.gui.resolve(strict=True)
    assert gui.is_file() and gui.suffix == ".deb"
    command.extend(["--mount", f"type=bind,source={gui},target=/tmp/rove-gui.deb,readonly"])
command.extend(["rove-build-linux:check-20260908", "sh", "-ec", script])
subprocess.run(command, check=True, timeout=120)
