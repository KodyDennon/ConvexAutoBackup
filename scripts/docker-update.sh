#!/usr/bin/env bash
# Rebuild the ConvexAutoBackup image from this checkout and restart the stack.
# Data lives in the `data` volume and is kept across updates.
#
#   scripts/docker-update.sh            # uses ~/.convex-autobackup/docker.env
#   ENV_FILE=/path/to/env scripts/docker-update.sh
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_FILE="${ENV_FILE:-$HOME/.convex-autobackup/docker.env}"

if [ ! -f "$ENV_FILE" ]; then
  echo "Env file $ENV_FILE not found." >&2
  exit 1
fi

cd "$REPO_DIR"
if [ "${SKIP_PULL:-0}" != "1" ] && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git pull --ff-only
fi

profiles=()
if grep -q '^TUNNEL_TOKEN=.\+' "$ENV_FILE"; then
  profiles=(--profile tunnel)
fi

docker compose --env-file "$ENV_FILE" "${profiles[@]}" build --pull
docker compose --env-file "$ENV_FILE" "${profiles[@]}" up -d --remove-orphans

echo "Waiting for the app to become healthy..."
for _ in $(seq 1 60); do
  status="$(docker inspect -f '{{.State.Health.Status}}' convex-autobackup 2>/dev/null || true)"
  if [ "$status" = "healthy" ]; then
    docker compose --env-file "$ENV_FILE" "${profiles[@]}" ps
    exit 0
  fi
  sleep 2
done
echo "App did not become healthy; check: docker compose logs app" >&2
exit 1
