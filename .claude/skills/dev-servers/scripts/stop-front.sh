#!/usr/bin/env bash
# ─── Stop Frontend ─────────────────────────────────────────────────────
source "$(cd "$(dirname "$0")" && pwd)/lib.sh"
svc_lookup front

if lsof -i :$SVC_PORT -P -n 2>/dev/null | grep -q LISTEN; then
  echo "🛑 Stopping Frontend..."
  pkill -f "$SVC_PATTERN" 2>/dev/null && echo "✅ Frontend stopped" || echo "❌ Failed to stop Frontend"
else
  echo "ℹ️  Frontend is not running"
fi
