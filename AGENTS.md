# AGENTS.md

This project is `ggx`, a Rust CLI for fast AI-powered git workflows. The main code lives in `src/`, user-facing docs are in `docs/`, and helper scripts are in `scripts/`.

Always keep documentation up to date as part of any change or implementation plan.

After any code or documentation modification, run exactly:

```sh
scripts/ci.sh
```

Do not run any other build, test, lint, format, or check command unless the user explicitly asks for it.
