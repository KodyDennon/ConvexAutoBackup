.PHONY: setup build test check fmt lint repo-size web-build web-test serve worker-policy mcp-health

setup:
	npm --prefix web install
	cargo fetch

build: web-build
	cargo build --workspace --all-targets

test: web-build
	cargo test --workspace --all-targets
	npm --prefix web test -- --run

check: repo-size web-build
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace --all-targets
	npm --prefix web test -- --run

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets -- -D warnings

repo-size:
	./scripts/ci/check-file-lines.sh 600

web-build:
	npm --prefix web run build

web-test:
	npm --prefix web test -- --run

serve:
	cargo run -p convex-autobackup -- serve

worker-policy:
	cargo run -p convex-autobackup-worker -- policy --json

mcp-health:
	printf '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"health","arguments":{}}}\n' | cargo run -p convex-autobackup-mcp
