#!/system/bin/sh
# Network facts from the Android-side Rove agent, read through its own local socket.
# Requires a debuggable build and adb access, for example:
#   adb push scripts/check-android-agent-network.sh /data/local/tmp/agent-net.sh
#   adb shell sh /data/local/tmp/agent-net.sh [endpoint ...]
# Read-only: it sends get_hello, list_networks, list_network_devices and
# test_network_peer requests and never changes networks, settings or services.
# The agent answers only while the app process (and therefore the agent) is alive.
SOCK="${ROVE_AGENT_SOCK:-/data/user/0/app.rove.desktop/rove/agent.sock}"
W=/data/local/tmp

if [ "$#" -gt 0 ]; then
  ENDPOINTS="$*"
else
  ENDPOINTS="tcp://10.1.2.240:22 tcp://10.1.2.242:22"
fi

ask() {
  op="$1"
  body="$2"
  cid="$3"
  nid="$4"
  path=""
  if [ -n "$nid" ]; then
    path=",\"path_parameters\":{\"network_id\":\"$nid\"}"
  fi
  if [ -n "$body" ]; then
    json="{\"kind\":\"request\",\"correlation_id\":\"$cid\",\"operation_id\":\"$op\"$path,\"body\":$body}"
  else
    json="{\"kind\":\"request\",\"correlation_id\":\"$cid\",\"operation_id\":\"$op\"$path}"
  fi
  printf '%s' "$json" > "$W/rove-req.json"
  l=$(wc -c < "$W/rove-req.json")
  printf "\\$(printf '%03o' $((l>>24&255)))\\$(printf '%03o' $((l>>16&255)))\\$(printf '%03o' $((l>>8&255)))\\$(printf '%03o' $((l&255)))" > "$W/rove-frame.bin"
  cat "$W/rove-req.json" >> "$W/rove-frame.bin"
  echo "===== $op $body ====="
  # toybox nc exits on stdin EOF, so hold stdin open until the response arrives.
  run-as app.rove.desktop sh -c "(cat $W/rove-frame.bin; sleep 4) | nc -U $SOCK" | tail -c +5
  echo
  rm -f "$W/rove-req.json" "$W/rove-frame.bin"
}

ask get_hello '' 44444444-4444-4444-4444-444444444440
ask list_networks '' 44444444-4444-4444-4444-444444444441

n=0
for endpoint in $ENDPOINTS; do
  n=$((n + 1))
  ask test_network_peer "{\"endpoint\":\"$endpoint\"}" "44444444-4444-4444-4444-44444444445$n"
done

if [ -n "$ROVE_NETWORK_ID" ]; then
  ask list_network_devices "" 44444444-4444-4444-4444-444444444499 "$ROVE_NETWORK_ID"
fi
