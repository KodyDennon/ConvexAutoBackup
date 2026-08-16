#!/usr/bin/env bash
set -euo pipefail

REPO_DIR="/home/user/projects/ConvexAutoBackup"
BIN_DIR="/home/user/.local/bin"
DATA_DIR="/home/user/.local/convex-autobackup"

echo "==> Fetching latest changes..."
git -C "$REPO_DIR" pull origin main || true

echo "==> Building web frontend assets..."
pnpm --prefix "$REPO_DIR/web" build
"$REPO_DIR/scripts/ci/sync-web-dist.sh"

echo "==> Compiling release binaries..."
cargo build --release --workspace --manifest-path "$REPO_DIR/Cargo.toml"

echo "==> Deploying binaries..."
mkdir -p "$BIN_DIR" "$DATA_DIR"
cp "$REPO_DIR/target/release/convex-autobackup" "$BIN_DIR/convex-autobackup"
cp "$REPO_DIR/target/release/convex-autobackup-worker" "$BIN_DIR/convex-autobackup-worker"
cp "$REPO_DIR/target/release/convex-autobackup-mcp" "$BIN_DIR/convex-autobackup-mcp"
cp "$REPO_DIR/target/release/convex-autobackup" "$DATA_DIR/convex-autobackup"

echo "==> Restarting systemd user unit..."
systemctl --user restart convex-autobackup.service || true

echo "==> ConvexAutoBackup update completed successfully."
