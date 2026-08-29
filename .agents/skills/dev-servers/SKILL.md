---
name: dev-servers
description: Checks the live health status of all local dev servers (backend Rust, frontend Nuxt, maquette mockup, PostgreSQL Docker). Provides per-service check/start/stop commands plus bulk JSON output. Use when the user asks about server status, whether a specific service is running, wants to start/stop a server, needs to debug a connection issue, or needs a health summary before testing.
---

# Dev Servers Skill

Monitor and manage the health of all local development services for the **Arcane Exchange** project.

## Overview

| Service                  | Port   | Type        | Endpoint                |
| ------------------------ | ------ | ----------- | ----------------------- |
| Backend (ae, Rust)       | `8080` | HTTP API    | `http://127.0.0.1:8080` |
| Frontend (Nuxt/Vue)      | `3000` | HTTP SPA    | `http://127.0.0.1:3000` |
| Maquette (static mockup) | `4000` | HTTP static | `http://127.0.0.1:4000` |
| PostgreSQL (Docker)      | `5432` | TCP         | `127.0.0.1:5432`        |

## Commands

All scripts are in `scripts/` directory relative to this skill.

### Check (health status)

Single service:

```bash
./scripts/check-back.sh          # Check backend (8080)
./scripts/check-front.sh         # Check frontend (3000)
./scripts/check-maquette.sh      # Check maquette (4000)
./scripts/check-db.sh            # Check PostgreSQL (5432)
```

All services at once:

```bash
./scripts/check-servers.sh              # Human-readable summary
./scripts/check-servers.sh --json        # Machine-parseable JSON
./scripts/check-servers.sh --json --full # JSON + PID, uptime, Docker info
```

### Start

```bash
./scripts/start-back.sh          # Start backend (foreground)
./scripts/start-front.sh        # Start frontend (foreground)
./scripts/start-maquette.sh     # Start maquette (foreground)
./scripts/start-db.sh           # Start PostgreSQL (Docker container)
```

### Stop

```bash
./scripts/stop-back.sh           # Stop backend (SIGTERM via PID)
./scripts/stop-front.sh         # Stop frontend (SIGTERM via PID)
./scripts/stop-maquette.sh      # Stop maquette (SIGTERM via PID)
./scripts/stop-db.sh            # Stop PostgreSQL (Docker container)
```

## Output Format

### Per-service check output

```
✅ Backend (PID: 81214) — running — port:8080 HTTP:200 — uptime:3h 42m 15s
❌ Frontend — stopped — port:3000
```

- `✅` = running, port open, HTTP 200
- `❌` = stopped, port closed
- Exit code `0` = running, `1` = stopped

### JSON (machine-readable)

When using `--json` on `check-servers.sh`, the script outputs valid JSON:

```json
{
  "checked_at": "14:23:01",
  "services": [
    {
      "name": "Backend",
      "host": "127.0.0.1",
      "port": 8080,
      "status": "running",
      "pid": 81214,
      "uptime": "3h 42m 15s",
      "http_code": "200",
      "endpoint": "http://127.0.0.1:8080"
    },
    {
      "name": "Frontend",
      "host": "127.0.0.1",
      "port": 3000,
      "status": "running",
      "pid": 81373,
      "uptime": "3h 41m 50s",
      "http_code": "200",
      "endpoint": "http://127.0.0.1:3000"
    },
    {
      "name": "Maquette",
      "host": "127.0.0.1",
      "port": 4000,
      "status": "running",
      "pid": 4608,
      "uptime": "2h 15m 30s",
      "http_code": "200",
      "endpoint": "http://127.0.0.1:4000"
    },
    {
      "name": "PostgreSQL",
      "host": "127.0.0.1",
      "port": 5432,
      "status": "running",
      "pid": null,
      "uptime": null,
      "http_code": null,
      "endpoint": "127.0.0.1:5432"
    }
  ]
}
```

## Troubleshooting

### Backend not responding

1. Check: `./scripts/check-back.sh`
2. Stop: `./scripts/stop-back.sh`
3. Restart: `mise run back`
4. `start-back.sh` runs in the foreground — errors print directly to that terminal, there is no log file to tail

### Frontend not responding

1. Check: `./scripts/check-front.sh`
2. Stop: `./scripts/stop-front.sh`
3. Restart: `mise run front`
4. Check Nuxt output for errors

### Maquette not responding

1. Check: `./scripts/check-maquette.sh`
2. Stop: `./scripts/stop-maquette.sh`
3. Restart: `mise run maquette`
4. Verify maquette directory exists at the project root

### PostgreSQL not reachable

1. Check: `./scripts/check-db.sh`
2. Stop: `./scripts/stop-db.sh`
3. Start: `./scripts/start-db.sh`
4. Test connection: `docker exec ccpt_local-database pg_isready`
5. Run migrations: `mise run migrate`

## Manual Checks (fallback if scripts are unavailable)

```bash
# Check if a port is open (any service)
nc -zv 127.0.0.1 <PORT>

# HTTP status code for web services
curl -s -o /dev/null -w "%{http_code}" --connect-timeout 2 http://127.0.0.1:<PORT>

# Find process using a port
lsof -i :<PORT> -P -n | grep LISTEN

# Check Docker PostgreSQL
docker ps --filter "name=ccpt_local-database"
docker exec ccpt_local-database-1 pg_isready

# Test DB connection from host
psql -h 127.0.0.1 -U postgres -c "SELECT 1"
```

## Integration Notes

### Compatible with

- **Pi** (`pi-coding-agent`) — auto-discovered as `/skill:dev-servers` via `.agents/skills/`
- **Claude Code** — auto-discovered via `.agents/skills/`
- Any harness that implements the [Agent Skills standard](https://agentskills.io/specification)

### When to use

- Before starting debugging: "is the backend even running?"
- After running `mise run clean`: verify all services come back up
- When the user reports a connection error or 500/404
- During onboarding: quick health overview of the local setup
- Before running tests: confirm DB and API are reachable
- When the user wants to start/stop a specific server
