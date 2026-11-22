#!/bin/sh
# Writes continuously to a probe table and to set_name, reconnecting on every iteration like a pool would. A write
# that fails (database down, read-only) is logged and the loop goes on. Every probe row it manages to commit is
# also recorded in the append-only probe_log table and in /control/committed.log, so that the test can check that
# nothing it committed on the old database is missing on the new one.
# With WRITER_WAIT_FOR_GO set, it waits for the /control/go file before its first write after the tables exist.
set -u

until psql -X -q -c "
  CREATE TABLE IF NOT EXISTS probe (id bigint PRIMARY KEY, val text NOT NULL, touched int NOT NULL DEFAULT 0);
  CREATE TABLE IF NOT EXISTS probe_log (id bigint PRIMARY KEY)" 2>/dev/null; do
  sleep 1
done
echo "writer: started on $PGHOST"

if [ -n "${WRITER_WAIT_FOR_GO:-}" ]; then
  until [ -e /control/go ]; do sleep 0.2; done
  echo "writer: go"
fi

while :; do
  if out=$(psql -X -q -t -A -c "
    WITH n AS (INSERT INTO probe (id, val) SELECT coalesce(max(id), 0) + 1, md5(random()::text) FROM probe_log RETURNING id),
         l AS (INSERT INTO probe_log (id) SELECT id FROM n)
    SELECT id FROM n;
    UPDATE probe SET val = md5(random()::text), touched = touched + 1 WHERE id = (SELECT id FROM probe ORDER BY random() LIMIT 1);
    DELETE FROM probe WHERE id = (SELECT id FROM probe ORDER BY random() LIMIT 1) AND (SELECT count(*) FROM probe) > 50;
    INSERT INTO set_name (set_code, name) VALUES ('ZZZTST', md5(random()::text)) ON CONFLICT (set_code) DO UPDATE SET name = excluded.name;
  " 2>&1); then
    echo "$out" >>/control/committed.log
  else
    echo "writer: write failed on $PGHOST: $out"
  fi
  sleep 0.1
done
