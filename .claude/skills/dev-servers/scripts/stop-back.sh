#!/usr/bin/env bash
# ─── Stop Backend ──────────────────────────────────────────────────────
source "$(cd "$(dirname "$0")" && pwd)/lib.sh"
svc_lookup back

if lsof -i :$SVC_PORT -P -n 2>/dev/null | grep -q LISTEN; then
  echo "🛑 Stopping Backend..."
  pkill -f "$SVC_PATTERN" 2>/dev/null && echo "✅ Backend stopped" || echo "❌ Failed to stop Backend"
else
  echo "ℹ️  Backend is not running"
fi
