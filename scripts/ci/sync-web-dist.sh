#!/usr/bin/env sh
set -eu

rm -rf crates/server/web-dist
mkdir -p crates/server/web-dist
cp -R web/dist/. crates/server/web-dist/
