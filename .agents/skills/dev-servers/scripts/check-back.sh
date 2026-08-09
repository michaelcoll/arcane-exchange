#!/usr/bin/env bash
# ─── Check Backend (port 8080) ─────────────────────────────────────────
source "$(cd "$(dirname "$0")" && pwd)/lib.sh"
svc_lookup back

# Check port
if port_open $SVC_PORT; then
  pid=$(lsof -i :$SVC_PORT -P -n 2>/dev/null | grep LISTEN | awk '{print $2}' | head -1)
  code=$(http_status "$SVC_ENDPOINT")
  uptime=$(ps -o etime= -p "$pid" 2>/dev/null | xargs || echo "unknown")
  echo "✅ Backend (PID: $pid) — running — port:$SVC_PORT HTTP:$code — uptime:$uptime"
  exit 0
else
  echo "❌ Backend — stopped — port:$SVC_PORT"
  exit 1
fi
