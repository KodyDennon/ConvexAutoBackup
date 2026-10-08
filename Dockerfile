FROM node:26.9-bookworm-slim AS web
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web ./
RUN npm run build

FROM rust:1.98-bookworm AS rust-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY --from=web /app/web/dist ./crates/server/web-dist
RUN cargo build --release --locked --workspace

FROM node:26.9-bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates curl tini \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=rust-builder /app/target/release/convex-autobackup /usr/local/bin/convex-autobackup
COPY --from=rust-builder /app/target/release/convex-autobackup-mcp /usr/local/bin/convex-autobackup-mcp
COPY --from=rust-builder /app/target/release/convex-autobackup-worker /usr/local/bin/convex-autobackup-worker
COPY packaging/docker-entrypoint.sh /usr/local/bin/convex-autobackup-entrypoint
RUN chmod +x /usr/local/bin/convex-autobackup-entrypoint

# Bake the pinned Convex CLI into the image so upgrades ship with the image
# and the container never needs to install packages at runtime.
RUN convex-autobackup --data-dir /opt/convex-autobackup runner install --json \
  && test -x /opt/convex-autobackup/runner/node_modules/.bin/convex

# /data is the only writable path; it is owned by the unprivileged `node` user (uid 1000).
RUN mkdir -p /data/home /data/backups && chown -R node:node /data

ENV CONVEX_AUTOBACKUP_BIND=0.0.0.0:8976 \
    CONVEX_AUTOBACKUP_DATA_DIR=/data \
    CONVEX_AUTOBACKUP_CONTAINER=1 \
    CONVEX_AUTOBACKUP_CONVEX_BIN=/opt/convex-autobackup/runner/node_modules/.bin/convex \
    HOME=/data/home \
    npm_config_cache=/data/home/.npm

USER node
EXPOSE 8976
VOLUME ["/data"]
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8976/api/v1/health >/dev/null || exit 1
ENTRYPOINT ["tini", "--", "convex-autobackup-entrypoint"]
CMD ["convex-autobackup", "supervise"]
