# Contributing to Yolo

Thanks for helping improve Yolo. Yolo is an Alpha project; small, focused
changes with tests and evidence are much easier to review than broad rewrites.

## Before you start

- Read [AGENTS.md](AGENTS.md), [docs/architecture.md](docs/architecture.md) and
  the contract relevant to your change.
- Search issues and discussions before opening a new one.
- For a large change, open an issue describing the invariant or user problem
  first. Do not start by adding a new abstraction layer.
- Security issues must follow [SECURITY.md](SECURITY.md), not the public issue
  tracker.

## Build and test

Requirements are Bazelisk with the pinned Bazel version and a Rust toolchain
selected by Bazel. Cargo is dependency metadata only.

```bash
bazel build --jobs=1 //core:yolo
bazel run --jobs=1 //core:yolo -- --help
bazel test --jobs=1 //core:tests
bazel test --jobs=1 //build_system/architecture:conformance_test
```

The web client is a Bun project outside the Bazel graph:

```bash
cd plugins/web/client
bun test
bun build browser/main.ts --target=browser --outdir /tmp/yolo-web-browser
```

Live browser acceptance requires an already-running plugin, a disposable shell
and Chrome DevTools. See
[the web client README](plugins/web/client/README.md). Automated terminal tests
must use private PTYs and must not change window focus.

## Change rules

- Keep every handwritten source and document under 200 lines. Split by
  responsibility, not by numeric part.
- Bazel defines production targets. Do not add a Cargo binary, workspace or
  release profile.
- Do not add dependency directories or committed build outputs.
- Preserve PTY custody, writer leases, runtime epochs and provider authority
  boundaries. A UI workaround must not weaken a runtime invariant.
- Keep browser payloads free of the management socket, Hub ID, lease grants,
  creation capabilities and plugin tokens.
- Keep provider-specific behavior in provider adapters; do not turn the TUI into
  a structured agent API.
- Update contracts, ADRs and tests together when a wire or lifecycle rule
  changes.

## Pull requests

Include:

1. The problem and the invariant being changed.
2. The smallest responsible design.
3. Tests or live evidence, with commands and results.
4. Known limits and follow-up work.

A pull request is ready when the focused tests, the architecture conformance
gate and the relevant live acceptance pass. Do not present a passing unit suite
as production approval.

## Commit style

Use a short imperative subject, for example `fix(web): release same-tab control
before prompt`. Keep unrelated changes in separate commits. By contributing, you
agree that your contribution is licensed under the repository's MIT license.
