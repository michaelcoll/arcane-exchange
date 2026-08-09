#!/usr/bin/env bash
# ─── Check Maquette (port 4000) ────────────────────────────────────────
source "$(cd "$(dirname "$0")" && pwd)/lib.sh"
svc_lookup maquette

if port_open $SVC_PORT; then
  pid=$(lsof -i :$SVC_PORT -P -n 2>/dev/null | grep LISTEN | awk '{print $2}' | head -1)
  code=$(http_status "$SVC_ENDPOINT")
  uptime=$(ps -o etime= -p "$pid" 2>/dev/null | xargs || echo "unknown")
  echo "✅ Maquette (PID: $pid) — running — port:$SVC_PORT HTTP:$code — uptime:$uptime"
  exit 0
else
  echo "❌ Maquette — stopped — port:$SVC_PORT"
  exit 1
fi
