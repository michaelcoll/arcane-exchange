#!/usr/bin/env bash
# Library: shared helpers for server management scripts

HERE="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$HERE/../../../.." && pwd)"

# Colours
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

# Helpers
port_open() {
  local port="$1"
  nc -z -w 2 127.0.0.1 "$port" 2>/dev/null
}

http_status() {
  local url="$1"
  curl -s -o /dev/null -w "%{http_code}" --connect-timeout 2 --max-time 5 "$url" 2>/dev/null || echo "000"
}

# docker ps always exits 0 even when the filter matches nothing, so the
# exit code alone can't be used to detect "container not running" — check
# the output instead.
docker_container_running() {
  local name_filter="$1"
  [[ -n "$(docker ps -q --filter "name=$name_filter" 2>/dev/null)" ]]
}

# Service lookup
# Sets: SVC_NAME, SVC_PORT, SVC_SCHEME, SVC_ENDPOINT, SVC_DIR, SVC_TASK, SVC_PATTERN, SVC_CONTAINER
svc_lookup() {
  local svc="$1"
  case "$svc" in
    back)
      SVC_NAME="Backend"
      SVC_PORT=8080
      SVC_SCHEME="http"
      SVC_ENDPOINT="http://127.0.0.1:8080"
      SVC_DIR="."
      SVC_TASK="back"
      SVC_PATTERN="target/debug/ae"
      ;;
    front)
      SVC_NAME="Frontend"
      SVC_PORT=3000
      SVC_SCHEME="http"
      SVC_ENDPOINT="http://127.0.0.1:3000"
      SVC_DIR="frontend-vue"
      SVC_TASK="front"
      SVC_PATTERN="nuxt"
      ;;
    maquette)
      SVC_NAME="Maquette"
      SVC_PORT=4000
      SVC_SCHEME="http"
      SVC_ENDPOINT="http://127.0.0.1:4000"
      SVC_DIR="frontend-vue"
      SVC_TASK="maquette"
      SVC_PATTERN="http-server"
      ;;
    db)
      SVC_NAME="PostgreSQL"
      SVC_PORT=5432
      SVC_SCHEME="tcp"
      SVC_ENDPOINT="127.0.0.1:5432"
      SVC_DIR="."
      SVC_CONTAINER="ccpt_local-database-1"
      ;;
    *)
      echo "Error: unknown service '$svc' (valid: back, front, maquette, db)" >&2
      exit 1
      ;;
  esac
}
