#!/usr/bin/env sh
target=$(basename "$(dirname "$0")")
printf 'target=%s\n' "$target"
for argument in "$@"; do
  printf 'arg=<%s>\n' "$argument"
done
cat
exit "${GGX_TEST_EXIT:-0}"
