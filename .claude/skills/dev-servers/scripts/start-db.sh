#!/usr/bin/env bash
# ─── Start PostgreSQL ──────────────────────────────────────────────────
source "$(cd "$(dirname "$0")" && pwd)/lib.sh"
svc_lookup db

echo "🚀 Starting PostgreSQL..."
echo "Command: docker start $SVC_CONTAINER"
echo ""
docker start "$SVC_CONTAINER" && echo "✅ PostgreSQL started" || echo "❌ Failed to start PostgreSQL"
