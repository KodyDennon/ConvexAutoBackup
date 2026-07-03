#!/usr/bin/env sh
set -eu

VERSION="${CONVEX_AUTOBACKUP_VERSION:-v0.1.0-beta.3}"
IMAGE_REGISTRY="${CONVEX_AUTOBACKUP_IMAGE:-ghcr.io/kodydennon/convex-autobackup:${VERSION}}"
INSTALL_DIR="${CONVEX_AUTOBACKUP_INSTALL_DIR:-$HOME/.convex-autobackup}"
PORT="${CONVEX_AUTOBACKUP_PORT:-8976}"

mkdir -p "$INSTALL_DIR"
cd "$INSTALL_DIR"

if [ ! -f .env ]; then
  if command -v openssl >/dev/null 2>&1; then
    MASTER_KEY="$(openssl rand -base64 48)"
  else
    MASTER_KEY="$(LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom | head -c 64)"
  fi
  cat > .env <<EOF
CONVEX_AUTOBACKUP_MASTER_KEY=$MASTER_KEY
CONVEX_AUTOBACKUP_BIND=0.0.0.0:8976
RUST_LOG=info
EOF
  chmod 600 .env
  echo "Generated $INSTALL_DIR/.env. Back up CONVEX_AUTOBACKUP_MASTER_KEY; losing it can make stored secrets unrecoverable."
fi

cat > docker-compose.yml <<EOF
services:
  convex-autobackup:
    image: ${IMAGE_REGISTRY}
    pull_policy: always
    ports:
      - "${PORT}:8976"
    env_file:
      - .env
    environment:
      CONVEX_AUTOBACKUP_DATA_DIR: /data
    volumes:
      - convex-autobackup-data:/data
    restart: unless-stopped

volumes:
  convex-autobackup-data:
EOF

docker compose pull
docker compose up -d

echo "Waiting for ConvexAutoBackup at http://localhost:${PORT}"
tries=0
until curl -fsS "http://localhost:${PORT}/api/v1/health" >/dev/null 2>&1; do
  tries=$((tries + 1))
  if [ "$tries" -gt 60 ]; then
    echo "Service did not become healthy. Run: cd $INSTALL_DIR && docker compose logs" >&2
    exit 1
  fi
  sleep 2
done

docker compose exec -T convex-autobackup convex-autobackup doctor --json

echo "ConvexAutoBackup is ready: http://localhost:${PORT}"
echo "Install directory: $INSTALL_DIR"
