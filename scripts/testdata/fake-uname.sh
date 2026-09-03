#!/usr/bin/env sh
case "${1:-}" in
  -s) printf '%s\n' "$GGX_TEST_OS" ;;
  -m) printf '%s\n' "$GGX_TEST_ARCH" ;;
  *) exit 1 ;;
esac
