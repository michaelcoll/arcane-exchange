#!/usr/bin/env bash
# ─── Start Frontend ────────────────────────────────────────────────────
source "$(cd "$(dirname "$0")" && pwd)/lib.sh"
svc_lookup front

echo "🚀 Starting Frontend..."
echo "Command: mise run $SVC_TASK"
echo "Note: this runs in the foreground."
echo ""
cd "$PROJECT_ROOT/$SVC_DIR" && mise run "$SVC_TASK"
