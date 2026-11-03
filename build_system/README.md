# Build System

This directory owns Yolo's repository-level build rules and build-time tools.
Product sources remain under `core/crates/` and first-party `plugins/` (TUI and
trusted web-client sources); build graph policy, coverage processing, and
cross-platform artifact plumbing live here.

The package boundaries are intentional:

- `architecture/` enforces repository and production architecture contracts.
  Mechanisms live in `foundation/`, `syntax/`, `contracts/`, `graph/` and
  `test_support/`; every policy lives under `checks/<domain>/`. Its
  target-ownership graph requires one real production Rust owner per source,
  prevents source aggregation across component packages, rejects cross-crate
  `#[path]`, and caps build-system crate size. Core has exactly one production
  Rust binary, `//core/crates/application/yolo:yolo`; entrypoint budgets, dependency
  boundaries and reviewed macro origins prevent hidden production executables.
- `release/` creates and validates release evidence. Delivery primitives,
  ordinary delivery, developer distribution, core gates, and Linux gates have
  separate crates behind thin public facades.
  `release/bazel_output/` owns retained output bindings and link-swap detection;
  gate orchestration consumes its public API without compiling its sources.
- `bazel/` owns first-party Bazel extensions and cross-repository contracts:
  `rust/` (Rust rules, isolated tests, dependency patches), `capnp/`,
  `cli_test_support/`, `cargo_metadata/`, and the shared `.bzl` contracts for
  version identity, lifecycle deadlines, process roles and SCM rights descriptors.
- `coverage/` is an aggregation boundary; `collector/`, `protocol/`, and
  `gate/` own collection, wire-format, and threshold enforcement separately.
  `instrumentation/` and `compiler/` integrate the pinned continuous-coverage
  producers.
- `platform/linux/` exposes verified remote-runtime artifacts to the build.
  Bundle schema 2 contains one `artifacts.runtime` descriptor and `bin/yolo`
  payload; `amd64/` and `arm64/` own the per-architecture payloads.
- `platform/macos/` owns only the macOS deployment floor. There is no AppKit
  client, native integration rule, app bundle, or inactive-window probe.

Build the binary with `bazel build --jobs=1 //core:yolo`, then run it with
`bazel run --jobs=1 //core:yolo -- [arguments]`. `yolo` opens the TUI;
`yolo internal <role>` starts independent services. See the
[single-executable architecture](../docs/single-executable-architecture.md).

The repository root discovers every `build_system/**` Bazel package with
`subpackages()` and requires its `architecture_check_data` target. Adding a
package without enrolling it in conformance therefore fails Bazel analysis
instead of silently creating an unchecked island.

First-party Bazel extensions must also live below `build_system/`. BUILD files
stay beside the targets they declare, while reusable macros and repository-wide
contracts do not belong in product source directories.

Every build-system component target and Bazel package may own at most 24
production Rust sources, and each package may contain at most 2,400 production
Rust lines. Rust tests and test-only libraries may add at most 24 test-owned
sources beyond that production code. Architecture and release roots have no
exception: they are thin aggregation targets whose implementation and tests
remain owned by their domain packages.
