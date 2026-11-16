#!/usr/bin/env bash
# ─── Dev Servers Health Check ───────────────────────────────────────────
# Usage: check-servers.sh [--json] [--full]
#   --json   Output valid JSON (for machine consumption)
#   --full   Extra detail: uptime, PID, Docker info (default: quick check)
#
# Works on Linux/macOS. Requires: bash, curl, nc (netcat), docker, jq (only for --json).

set -euo pipefail

source "$(cd "$(dirname "$0")" && pwd)/lib.sh"

# ── Extra colours / formatting (RED/GREEN/YELLOW/NC come from lib.sh) ───
CYAN='\033[0;36m'
DIM='\033[0;90m'

# ── Parsed args ────────────────────────────────────────────────────────
OUTPUT_JSON=false
FULL_CHECK=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)  OUTPUT_JSON=true; shift ;;
    --full)  FULL_CHECK=true; shift ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

ts() { date '+%H:%M:%S'; }

# ── Service definitions ───────────────────────────────────────────────
# Format: NAME|PORT|ENDPOINT
declare -a SERVICES=(
  "Backend|8080|http://127.0.0.1:8080"
  "Frontend|3000|http://127.0.0.1:3000"
  "Maquette|4000|http://127.0.0.1:4000"
  "PostgreSQL|5432|127.0.0.1:5432"
)

DB_CONTAINER="ccpt_local-database-1"

# ── Quick JSON output ────────────────────────────────────────────────
if $OUTPUT_JSON; then
  tmpfile=$(mktemp)
  echo '[]' > "$tmpfile"

  for svc in "${SERVICES[@]}"; do
    IFS='|' read -r NAME PORT ENDPOINT <<< "$svc"
    status="unknown"

    # Port check
    if port_open "$PORT"; then
      status="running"
    else
      status="stopped"
    fi

    pid=""
    uptime=""
    http_code=""

    if [[ "$status" == "running" && "$NAME" != "PostgreSQL" ]]; then
      pid=$(lsof -i :"$PORT" -P -n 2>/dev/null | grep LISTEN | awk '{print $2}' | head -1 || echo "")
      http_code=$(http_status "$ENDPOINT")
      if [[ -n "$pid" ]]; then
        uptime=$(ps -o etime= -p "$pid" 2>/dev/null | xargs || echo "unknown")
      fi
    fi

    # Convert empty values to null for JSON
    [[ -z "$pid" ]] && pid="null"
    [[ -z "$uptime" ]] && uptime="null"
    [[ -z "$http_code" ]] && http_code="null"

    # Build base JSON object
    obj=$(jq -n \
      --arg name "$NAME" \
      --arg host "127.0.0.1" \
      --argjson port "$PORT" \
      --arg status "$status" \
      --arg pid "$pid" \
      --arg uptime "$uptime" \
      --arg http_code "$http_code" \
      --arg endpoint "$ENDPOINT" \
      '{name: $name, host: $host, port: $port, status: $status,
        pid: (if $pid == "null" then null else $pid end),
        uptime: (if $uptime == "null" then null else $uptime end),
        http_code: (if $http_code == "null" then null else $http_code end),
        endpoint: $endpoint}')

    # Add full-mode fields
    if $FULL_CHECK && [[ "$NAME" == "Backend" || "$NAME" == "PostgreSQL" ]]; then
      if docker_container_running "$DB_CONTAINER"; then
        db_detail="postgresql running (Docker container: $DB_CONTAINER)"
      else
        db_detail="postgresql not running"
      fi
      obj=$(echo "$obj" | jq --arg dd "$db_detail" '. + {db_detail: $dd}')
    fi

    # Append to array
    jq --argjson obj "$obj" '. += [$obj]' "$tmpfile" > "$tmpfile.json" && mv "$tmpfile.json" "$tmpfile"
  done

  # Final output
  jq -n \
    --arg checked_at "$(ts)" \
    --slurpfile services "$tmpfile" \
    '{checked_at: $checked_at, services: $services[0]}'

  rm -f "$tmpfile"
  exit 0
fi

# ── Human-readable output ────────────────────────────────────────────
echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║       ${NC}${DIM}⚡ Arcane Exchange — Dev Servers Health Check${NC}    ${CYAN}        ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

total=0
running=0
stopped=0

for svc in "${SERVICES[@]}"; do
  IFS='|' read -r NAME PORT ENDPOINT <<< "$svc"
  total=$((total + 1))

  if port_open "$PORT"; then
    status="running"
    color="$GREEN"
    running=$((running + 1))

    # Extra details
    extra=""
    pid=""
    if [[ "$NAME" == "PostgreSQL" ]]; then
      if docker_container_running "$DB_CONTAINER"; then
        extra="Docker: $DB_CONTAINER"
      else
        extra="no Docker container"
        status="warning"
        color="$YELLOW"
      fi
    else
      pid=$(lsof -i :"$PORT" -P -n 2>/dev/null | grep LISTEN | awk '{print $2}' | head -1 || echo "")
      code=$(http_status "$ENDPOINT")
      extra="PID:${pid:-n/a} HTTP:$code"
    fi

    echo -e "  ${color}●${NC} ${NAME:0:12} : ${color}running${NC}  port:$PORT  pid:${pid:-n/a}  $extra"
  else
    color="$RED"
    stopped=$((stopped + 1))
    echo -e "  ${color}○${NC} ${NAME:0:12} : ${color}stopped${NC}  port:$PORT"
  fi
done

echo ""
# PostgreSQL extra: try psql if available
if command -v psql &>/dev/null; then
  echo -e "  ${DIM}↳ psql check: ${NC}"
  if psql -h 127.0.0.1 -U postgres -l &>/dev/null; then
    echo -e "    ${GREEN}● Database reachable${NC}"
  else
    echo -e "    ${RED}○ Database unreachable${NC}"
  fi
fi

# Summary
echo ""
echo -e "  ${DIM}Summary: $running running / $stopped stopped / $total total${NC}"

exit 0
