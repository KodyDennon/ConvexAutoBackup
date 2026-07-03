FROM node:26-bookworm-slim AS web
WORKDIR /app/web
COPY web/package.json web/package-lock.json* ./
RUN npm install
COPY web ./
RUN npm run build

FROM rust:1.95-bookworm AS rust-builder
WORKDIR /app
COPY Cargo.toml ./
COPY crates ./crates
COPY --from=web /app/web/dist ./crates/server/web-dist
RUN cargo build --release --workspace

FROM node:26-bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates curl \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=rust-builder /app/target/release/convex-autobackup /usr/local/bin/convex-autobackup
COPY --from=rust-builder /app/target/release/convex-autobackup-mcp /usr/local/bin/convex-autobackup-mcp
COPY --from=rust-builder /app/target/release/convex-autobackup-worker /usr/local/bin/convex-autobackup-worker
COPY packaging/docker-entrypoint.sh /usr/local/bin/convex-autobackup-entrypoint
RUN chmod +x /usr/local/bin/convex-autobackup-entrypoint
ENV CONVEX_AUTOBACKUP_BIND=0.0.0.0:8976
ENV CONVEX_AUTOBACKUP_DATA_DIR=/data
EXPOSE 8976
VOLUME ["/data"]
ENTRYPOINT ["convex-autobackup-entrypoint"]
CMD ["convex-autobackup", "supervise"]
