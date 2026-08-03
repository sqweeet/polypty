#!/usr/bin/env bash
set -euo pipefail

readonly max_lines=150
failed=0

while IFS= read -r -d '' file; do
  lines=$(wc -l < "$file")
  if (( lines > max_lines )); then
    printf '%s: %d lines (maximum %d)\n' "$file" "$lines" "$max_lines" >&2
    failed=1
  fi
done < <(find src tests -type f -name '*.rs' -print0 2>/dev/null | sort -z)

if (( failed != 0 )); then
  printf 'architecture check failed\n' >&2
  exit 1
fi

printf 'architecture check passed: every Rust file is <= %d lines\n' "$max_lines"
