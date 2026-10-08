#!/usr/bin/env sh
set -eu

: "${CONVEX_AUTOBACKUP_DATA_DIR:=/data}"
export CONVEX_AUTOBACKUP_DATA_DIR

# Offline recovery (`convex-autobackup decrypt`) only needs the backup passphrase.
case " $* " in
  *" decrypt "*) exec "$@" ;;
esac

if [ -z "${CONVEX_AUTOBACKUP_MASTER_KEY:-}" ]; then
  echo "CONVEX_AUTOBACKUP_MASTER_KEY is required. Keep this key backed up; losing it can make stored secrets unrecoverable." >&2
  exit 1
fi

mkdir -p "$CONVEX_AUTOBACKUP_DATA_DIR/home" "$CONVEX_AUTOBACKUP_DATA_DIR/backups"

# Images built from this repo bake the Convex CLI in and set CONVEX_AUTOBACKUP_CONVEX_BIN.
# Older images or custom builds fall back to installing it into the data volume.
if [ -z "${CONVEX_AUTOBACKUP_CONVEX_BIN:-}" ] && [ ! -x "$CONVEX_AUTOBACKUP_DATA_DIR/runner/node_modules/.bin/convex" ]; then
  echo "Installing pinned Convex CLI runner into $CONVEX_AUTOBACKUP_DATA_DIR/runner"
  convex-autobackup --data-dir "$CONVEX_AUTOBACKUP_DATA_DIR" runner install --json
fi

exec "$@"
