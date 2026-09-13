# AGENTS.md

This project is `ggx`, a Rust CLI for fast AI-powered git workflows. The main code lives in `src/`, user-facing docs are in `docs/`, and helper scripts are in `scripts/`.

Always keep documentation up to date as part of any change or implementation plan.

## Tests

Tests live in `#[cfg(test)]` modules next to the code. When you add or change behavior, add or update tests for it.

Write tests that pin down real behavior: the pure logic behind each command (parsing, validation, prompt rendering, formatting) and each error branch with its exact message. Do not write tests that:

- check that a removed feature or argument is still gone;
- copy the implementation into the test (for example asserting a full argument list constant by constant);
- repeat a case already covered through a different input;
- exercise clap, serde, or the standard library instead of this project's logic;
- need a real terminal, network, or the `git`, `gh`, `curl`, or provider CLIs.

After any code or documentation modification, run exactly:

```sh
scripts/ci.sh
```

Do not run any other build, test, lint, format, or check command unless the user explicitly asks for it.
