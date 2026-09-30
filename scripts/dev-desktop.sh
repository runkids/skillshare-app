#!/usr/bin/env bash
set -euo pipefail

pnpm install --frozen-lockfile

Xvfb "$DISPLAY" -screen 0 1600x1000x24 -nolisten tcp &
for attempt in {1..50}; do
  if xdpyinfo -display "$DISPLAY" >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done
xdpyinfo -display "$DISPLAY" >/dev/null

openbox &
tint2 &
x11vnc -display "$DISPLAY" -rfbport 5900 -localhost -nopw -forever -shared &
websockify --web /usr/share/novnc 0.0.0.0:6080 localhost:5900 &

echo 'Desktop: http://localhost:6080/vnc.html?autoconnect=true&resize=scale'
exec pnpm dev:tauri --config '{"build":{"beforeDevCommand":"pnpm dev --host 0.0.0.0"}}'
