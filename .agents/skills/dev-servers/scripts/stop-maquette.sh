#!/usr/bin/env bash
# ─── Stop Maquette ─────────────────────────────────────────────────────
source "$(cd "$(dirname "$0")" && pwd)/lib.sh"
svc_lookup maquette

if lsof -i :$SVC_PORT -P -n 2>/dev/null | grep -q LISTEN; then
  echo "🛑 Stopping Maquette..."
  pkill -f "$SVC_PATTERN" 2>/dev/null && echo "✅ Maquette stopped" || echo "❌ Failed to stop Maquette"
else
  echo "ℹ️  Maquette is not running"
fi
