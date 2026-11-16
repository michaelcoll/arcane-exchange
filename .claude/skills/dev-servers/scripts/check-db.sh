#!/usr/bin/env bash
# ─── Check PostgreSQL (port 5432) ──────────────────────────────────────
source "$(cd "$(dirname "$0")" && pwd)/lib.sh"
svc_lookup db

if port_open $SVC_PORT; then
  if docker_container_running "$SVC_CONTAINER"; then
    if docker exec "$SVC_CONTAINER" pg_isready -q 2>/dev/null; then
      echo "✅ PostgreSQL — running — port:$SVC_PORT — Docker: $SVC_CONTAINER"
      exit 0
    else
      echo "⚠️  PostgreSQL — port open but pg_isready failed"
      exit 1
    fi
  else
    echo "⚠️  PostgreSQL — port open but no Docker container running"
    exit 1
  fi
else
  echo "❌ PostgreSQL — stopped — port:$SVC_PORT"
  exit 1
fi
