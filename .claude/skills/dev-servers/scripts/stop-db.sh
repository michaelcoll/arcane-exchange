#!/usr/bin/env bash
# ─── Stop PostgreSQL ───────────────────────────────────────────────────
source "$(cd "$(dirname "$0")" && pwd)/lib.sh"
svc_lookup db

if docker_container_running "$SVC_CONTAINER"; then
  echo "🛑 Stopping PostgreSQL..."
  docker stop "$SVC_CONTAINER" && echo "✅ PostgreSQL stopped" || echo "❌ Failed to stop PostgreSQL"
else
  echo "ℹ️  PostgreSQL is not running"
fi
