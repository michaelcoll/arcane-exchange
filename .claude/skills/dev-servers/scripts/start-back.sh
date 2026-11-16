#!/usr/bin/env bash
# ─── Start Backend ─────────────────────────────────────────────────────
source "$(cd "$(dirname "$0")" && pwd)/lib.sh"
svc_lookup back

echo "🚀 Starting Backend..."
echo "Command: mise run $SVC_TASK"
echo "Note: this runs in the foreground."
echo ""
cd "$PROJECT_ROOT/$SVC_DIR" && mise run "$SVC_TASK"
