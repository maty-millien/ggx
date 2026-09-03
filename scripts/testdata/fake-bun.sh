#!/usr/bin/env sh
if [ "${1:-}" = "pm" ] && [ "${2:-}" = "bin" ] && [ "${3:-}" = "--global" ]; then
  printf '%s\n' "$GGX_TEST_BUN_BIN"
  exit
fi

{
  printf 'bun'
  for argument in "$@"; do
    printf ' %s' "$argument"
  done
  printf '\n'
} >> "$GGX_TEST_UPDATE_LOG"
exit "${GGX_TEST_UPDATE_FAIL:-0}"
