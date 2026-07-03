#!/usr/bin/env sh
set -eu

: "${CONVEX_AUTOBACKUP_DATA_DIR:=/data}"
export CONVEX_AUTOBACKUP_DATA_DIR

if [ -z "${CONVEX_AUTOBACKUP_MASTER_KEY:-}" ]; then
  echo "CONVEX_AUTOBACKUP_MASTER_KEY is required. Keep this key backed up; losing it can make stored secrets unrecoverable." >&2
  exit 1
fi

if [ ! -x "$CONVEX_AUTOBACKUP_DATA_DIR/runner/node_modules/.bin/convex" ]; then
  echo "Installing pinned Convex CLI runner into $CONVEX_AUTOBACKUP_DATA_DIR/runner"
  convex-autobackup --data-dir "$CONVEX_AUTOBACKUP_DATA_DIR" runner install --json
fi

exec "$@"
