#!/usr/bin/env sh
if [ "${1:-}" = "root" ] && [ "${2:-}" = "--global" ]; then
  printf '%s/lib/node_modules\n' "$GGX_TEST_NPM_PREFIX"
  exit
fi

{
  printf 'npm'
  for argument in "$@"; do
    printf ' %s' "$argument"
  done
  printf '\n'
} >> "$GGX_TEST_UPDATE_LOG"
exit "${GGX_TEST_UPDATE_FAIL:-0}"
