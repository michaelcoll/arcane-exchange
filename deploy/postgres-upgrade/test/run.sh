#!/usr/bin/env bash
# End-to-end test of the PostgreSQL 18 -> 19 upgrade (see ../README.md), run by `mise run test-pg-upgrade`.
#
# It plays the three deploys with the real docker-compose files, in its own compose project, with no host port and
# its own temporary directories. A synthetic writer stands in for the backend. The seed is read from the developer's
# local database container (read only: pg_dump).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
pkg=$(dirname "$here")
root=$(cd "$pkg/../.." && pwd)
PROJECT=pg_upgrade_test
SEED_CONTAINER=${PG_UPGRADE_SEED_CONTAINER:-ccpt_local-database-1}

work=$(mktemp -d "${TMPDIR:-/tmp}/pg-upgrade-test.XXXXXX")
CONTROL_DIR="$work/control"
export CCPT_DATA_DIR="$work/ccpt" CCPT_DB_DIR="$work/ccpt/db" WRITER_CONTROL_DIR="$CONTROL_DIR"
# The compose files interpolate these for the backend and frontend, which the test never starts.
export CLERK_FRONTEND_API_URL=unused CLERK_PUBLISHABLE_KEY=unused CLERK_SECRET_KEY=unused

step() { printf '\n==> %s\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }

cleanup() {
  local status=$?
  set +e
  step "Tearing down (project $PROJECT, $work)"
  if [ "$status" != "0" ]; then
    docker compose -p "$PROJECT" ps -a 2>&1 | tail -n 20
    for service in pg-replicate pg-cutover pg-guard writer; do
      docker compose -p "$PROJECT" logs "$service" 2>&1 | tail -n 40
    done
  fi
  docker compose -p "$PROJECT" down -v --remove-orphans >/dev/null 2>&1
  # The containers create files owned by another uid in the bind-mounted folders: empty them from a container.
  docker run --rm -v "$work:/work" postgres:19 sh -c 'rm -rf /work/* /work/.[!.]*' >/dev/null 2>&1
  rmdir "$work" 2>/dev/null
  exit "$status"
}
trap cleanup EXIT

# dc <stage> <docker compose args>: stage 0 is the state before deploy 1, stages 1 to 3 are the three deploys.
dc() {
  local stage=$1
  shift
  local files db_host=postgres wait_for_go=
  case $stage in
    0) files=(-f "$root/docker-compose.yml" -f "$here/writer.yml" -f "$here/stage1.yml" -f "$here/stage0.yml") ;;
    1) files=(-f "$root/docker-compose.yml" -f "$here/writer.yml" -f "$here/stage1.yml") ;;
    2) files=(-f "$pkg/docker-compose.deploy-2.yml" -f "$here/writer.yml" -f "$here/stage2.yml") db_host=postgres19 wait_for_go=1 ;;
    3) files=(-f "$pkg/docker-compose.deploy-3.yml" -f "$here/writer.yml" -f "$here/stage3.yml") ;;
  esac
  WRITER_DB_HOST=$db_host WRITER_WAIT_FOR_GO=$wait_for_go docker compose -p "$PROJECT" --project-directory "$root" "${files[@]}" "$@"
}

# sql <stage> <service> <psql args>
sql() {
  local stage=$1 service=$2
  shift 2
  dc "$stage" exec -T "$service" psql -U postgres -X -t -A -v ON_ERROR_STOP=1 "$@"
}

# wait_for <description> <timeout seconds> <command...>
wait_for() {
  local description=$1 timeout=$2 waited=0
  shift 2
  until "$@" >/dev/null 2>&1; do
    [ "$waited" -lt "$timeout" ] || fail "timeout (${timeout}s) waiting for: $description"
    sleep 2
    waited=$((waited + 2))
  done
}

# wait_exit <stage> <service>: waits for a one-shot service to stop and prints its exit code.
wait_exit() {
  local container
  container=$(dc "$1" ps -aq "$2")
  docker wait "$container"
}

probe_md5() { sql "$1" "$2" -c "SELECT md5(string_agg(id || ':' || val || ':' || touched, ',' ORDER BY id)) FROM probe"; }
set_name_md5() { sql "$1" "$2" -c "SELECT md5(string_agg(set_code || ':' || name || ':' || coalesce(released_at::text, ''), ',' ORDER BY set_code)) FROM set_name"; }
probe_count() { sql "$1" "$2" -c "SELECT count(*) FROM probe"; }
probe_max() { sql "$1" "$2" -c "SELECT max(id) FROM probe"; }
subscription_ready() { [ "$(sql 1 postgres19 -c "SELECT count(*) FROM pg_subscription_rel WHERE srsubstate <> 'r'")" = "0" ] \
  && [ "$(sql 1 postgres19 -c "SELECT count(*) FROM pg_subscription_rel")" != "0" ]; }
probe_filled() { [ "$(probe_count 0 postgres)" -gt 10 ]; }
logs_contain() { dc "$1" logs "$2" 2>&1 | grep -q "$3"; }
writer_progressing() {
  local before
  before=$(sql "$1" "$2" -c "SELECT count(*) FROM probe_log")
  sleep 2
  [ "$(sql "$1" "$2" -c "SELECT count(*) FROM probe_log")" -gt "$before" ]
}

step "Preparing the prod-like folders in $work"
docker compose -p "$PROJECT" down -v --remove-orphans >/dev/null 2>&1 || true
docker inspect -f '{{.State.Running}}' "$SEED_CONTAINER" 2>/dev/null | grep -q true \
  || fail "seed container $SEED_CONTAINER is not running (set PG_UPGRADE_SEED_CONTAINER to a running PostgreSQL holding the dev database)"
mkdir -p "$CCPT_DATA_DIR/card-images" "$CCPT_DB_DIR" "$CONTROL_DIR"
echo "dummy card image" >"$CCPT_DATA_DIR/card-images/dummy.webp"
images_sum=$(cksum <"$CCPT_DATA_DIR/card-images/dummy.webp")

step "Stage 0: PostgreSQL 18 as in production today, seeded from $SEED_CONTAINER"
dc 0 up -d --wait postgres
[ "$(sql 0 postgres -c "SHOW wal_level")" = "replica" ] || fail "stage 0 should start with wal_level=replica"
# The dump creates the public schema itself.
sql 0 postgres -c "DROP SCHEMA public"
docker exec "$SEED_CONTAINER" pg_dump -U postgres -d postgres -n public -e pg_trgm --no-owner --no-privileges \
  | dc 0 exec -T postgres psql -U postgres -X -q -v ON_ERROR_STOP=1 -o /dev/null
echo "seeded: $(sql 0 postgres -c "SELECT count(*) FROM pg_tables WHERE schemaname = 'public'") tables, $(sql 0 postgres -c "SELECT pg_size_pretty(pg_database_size('postgres'))")"

step "Starting the writer against the old database"
dc 0 up -d writer
wait_for "probe table filled" 60 probe_filled

step "Deploy 1: docker compose up -d (twice, to prove idempotence)"
dc 1 up -d
[ "$(wait_exit 1 pg-replicate)" = "0" ] || fail "pg-replicate failed"
logs_contain 1 pg-replicate "creating subscription" || fail "pg-replicate did not create the subscription"
[ "$(sql 1 postgres -c "SHOW wal_level")" = "logical" ] || fail "wal_level should be logical on the old database"
wait_for "initial copy" 1200 subscription_ready
dc 1 up -d
wait_for "second pg-replicate run" 60 logs_contain 1 pg-replicate "nothing to do"
[ "$(sql 1 postgres19 -c "SELECT count(*) FROM pg_subscription")" = "1" ] || fail "expected exactly one subscription"
wait_for "writer replicated to 19" 60 writer_progressing 1 postgres19
echo "replication live: probe max id $(probe_max 1 postgres) on 18, $(probe_max 1 postgres19) on 19"

step "Deploy 2: docker compose up -d while the old writer is still running (Portainer applies the new file as is)"
old_writer=$(dc 1 ps -q writer)
[ "$(docker inspect -f '{{.State.Running}}' "$old_writer")" = "true" ] || fail "old writer should be running"
docker logs -f "$old_writer" >"$work/old-writer.log" 2>&1 &
# Records which comes first: compose stopping the old writer, or pg-cutover starting.
: >"$work/order"
{ docker wait "$old_writer" >/dev/null; echo old-writer-stopped >>"$work/order"; } &
{ until [ -n "$(docker ps -q --filter "label=com.docker.compose.project=$PROJECT" --filter label=com.docker.compose.service=pg-cutover --filter status=running)" ]; do sleep 0.1; done; echo cutover-started >>"$work/order"; } &
rm -f "$CONTROL_DIR/go"
dc 2 up -d --remove-orphans
[ "$(wait_exit 2 pg-cutover)" = "0" ] || fail "pg-cutover failed"
dc 2 logs pg-cutover
wait
[ "$(dc 2 ps -q writer)" != "$old_writer" ] || fail "the writer should have been recreated against postgres19"
# Compose recreates the writer (stops the old one) before it starts pg-cutover: the old backend never meets the read-only database.
[ "$(head -n 1 "$work/order")" = "old-writer-stopped" ] || fail "expected the old writer to stop before pg-cutover starts, got: $(tr '\n' ' ' <"$work/order")"
grep -q "writer: write failed" "$work/old-writer.log" || fail "the old writer should have survived the restart of the old database (deploy 1) and kept looping"
echo "old writer stopped before pg-cutover started; it had survived the restart of 18 and kept looping"
[ "$(sql 2 postgres -c "SHOW default_transaction_read_only")" = "on" ] || fail "old database should be read-only"
if sql 2 postgres -c "INSERT INTO probe (id, val) VALUES (-1, 'x')" 2>/dev/null; then fail "old database accepted a write"; fi
[ "$(probe_md5 2 postgres)" = "$(probe_md5 2 postgres19)" ] || fail "probe checksum differs between 18 and 19"
[ "$(set_name_md5 2 postgres)" = "$(set_name_md5 2 postgres19)" ] || fail "set_name checksum differs between 18 and 19"
[ "$(sql 2 postgres19 -c "SELECT count(*) FROM pg_subscription")" = "0" ] || fail "subscription not dropped"
[ "$(sql 2 postgres -c "SELECT count(*) FROM pg_replication_slots")" = "0" ] || fail "replication slot not dropped"
[ "$(sql 2 postgres -c "SELECT count(*) FROM pg_publication")" = "0" ] || fail "publication not dropped"
echo "probe md5 equal on 18 and 19: $(probe_md5 2 postgres19)"
committed=$(wc -l <"$CONTROL_DIR/committed.log" | tr -d ' ')
missing=$(sql 2 postgres19 -c "SELECT count(*) FROM unnest('{$(paste -sd, "$CONTROL_DIR/committed.log")}'::bigint[]) AS c(id) WHERE id NOT IN (SELECT id FROM probe_log)")
[ "$committed" -gt 0 ] && [ "$missing" = "0" ] || fail "rows committed by the old writer are missing on 19 ($missing of $committed)"
echo "all $committed rows committed by the old writer are on 19"
touch "$CONTROL_DIR/go"
wait_for "writer released against 19" 60 writer_progressing 2 postgres19
dc 2 up -d
wait_for "second pg-cutover run" 60 logs_contain 2 pg-cutover "already cut over"

step "Deploy 3, mistake: docker compose up -d without stopping postgres19 (the guard must refuse)"
dc 2 stop writer
if dc 3 up -d 2>&1; then fail "deploy 3 started while postgres19 was still running"; fi
[ "$(docker inspect -f '{{.State.Running}}' "$(dc 2 ps -q postgres19)")" = "true" ] || fail "postgres19 should be untouched"
dc 3 logs pg-guard

step "Deploy 3, runbook: stop postgres19, then docker compose up -d --remove-orphans"
md5_before=$(probe_md5 2 postgres19)
dc 2 stop postgres19
dc 3 up -d --remove-orphans --wait postgres
[ "$(sql 3 postgres -c "SHOW server_version_num" | cut -c1-2)" = "19" ] || fail "postgres service is not PostgreSQL 19"
[ "$(probe_md5 3 postgres)" = "$md5_before" ] || fail "probe checksum changed across the rename"
[ "$(dc 3 ps -a --format '{{.Service}}' | sort | tr '\n' ' ')" = "pg-guard postgres writer " ] || fail "orphans were not removed"
dc 3 up -d
wait_for "writer on the renamed service" 60 writer_progressing 3 postgres
[ -f "$CCPT_DB_DIR/19/docker/PG_VERSION" ] || fail "19 data is not in the ccpt/db folder"
[ "$(cat "$CCPT_DATA_DIR/pgdata/PG_VERSION" 2>/dev/null || docker run --rm -v "$work:/work" postgres:19 cat /work/ccpt/pgdata/PG_VERSION)" = "18" ] || fail "old data folder should be untouched"
[ "$(cksum <"$CCPT_DATA_DIR/card-images/dummy.webp")" = "$images_sum" ] || fail "card-images changed"

step "PASSED"
