#!/usr/bin/env bash
set -euo pipefail

limit="${1:-600}"
paths=(crates web/src)
failed=0

while IFS= read -r -d '' file; do
  lines="$(wc -l < "$file" | tr -d ' ')"
  if (( lines > limit )); then
    printf 'File exceeds %s lines: %s (%s)\n' "$limit" "$file" "$lines" >&2
    failed=1
  fi
done < <(find "${paths[@]}" \
  -path '*/target' -prune -o \
  -path '*/node_modules' -prune -o \
  -path '*/dist' -prune -o \
  -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' -o -name '*.css' \) \
  -print0)

exit "$failed"
