# Architecture Enforcement

`//build_system/architecture:check` loads the tracked repository through Bazel
runfiles and returns a non-zero status for every policy violation. The
`//build_system/architecture:tests` suite combines independently owned policy
crate tests with the checker conformance test.

## Rust target ownership

The `target_ownership` crate tokenizes BUILD and Starlark sources, expands local
variables, loaded source lists, globs, aliases, and filegroups, and then builds
a production-source ownership graph. It enforces these contracts:

- every production Rust source has exactly one non-test-only `rust_library` or
  `rust_binary` owner declared directly in a BUILD file;
- every build-system production source resides below its owning package's `src`,
  including architecture and release facade sources;
- core and build-system targets cannot compile another component's production sources;
- production source filegroups cannot aggregate another package;
- production `#[path]` attributes cannot cross crate packages;
- every build-system production target owns at most 24 Rust sources, and every
  build-system, core, or plugin package stays within 24 production Rust sources
  and 2,400 lines, with recorded core/plugin package sizes failing on growth
  until they are split;
- every build-system `rust_test`, `isolated_rust_test`, and test-only Rust library adds at
  most 24 test-owned sources beyond its separately budgeted production code;
- every test or support source under `core/`, `build_system/`, and
  `integration_tests/` belongs to a local test or test-only Rust target;
- Rust test targets cannot compile another Bazel package's sources or combine
  `unit_tests/` and `integration_tests/` in one target;
- unit and integration sources owned by targets with an explicit `crate_root`
  must be reachable from an actual test root, not merely included by `srcs`;
- build-system production Rust targets cannot compile test-owned sources, even
  when a test-only target also owns them or a filegroup hides their paths;
- opaque build-system `srcs` expressions fail closed instead of evading budgets;
- architecture and release facades receive no size exception;
- parent globs stop at child BUILD boundaries, so a facade cannot silently
  reclaim extracted implementation or test sources.

Dependency list comprehensions do not suppress ownership checks on a rule's
explicit `srcs`. Comprehensions used as `srcs` remain unresolved. Nested rule
calls cannot manufacture owners, and malformed or unclosed calls claim no sources.
The source-count and package-line budgets apply to build-system crates and,
through recorded baselines, to core and plugin packages. A recorded package
may not grow past its current sources or lines.

The production/test classification for path-only checks is shared through
`foundation::source_path`, so target ownership, include composition, and source
policy agree on test directories and file names. Test reachability reuses the
Rust module graph independently for each target and
restricts traversal to that target's declared source set. It follows direct
modules, literal `#[path]`, inline modules, and literal `include!`. BUILD feature
lists resolve through local assignments, loaded aliases, list composition, and
reassignment order. `cfg(test)` is enabled only for `rust_test` and
`isolated_rust_test`, never merely because a library has `testonly = True`.
Boolean `all`, `any`, and `not` conditions are evaluated for known test/features;
platform predicates and opaque feature expressions remain potentially enabled.
Conditional module paths, dynamic includes, and unresolved graph edges are
reported as uncertain when they prevent proving a test source reachable.
Targets without an explicit crate root, dependency feature unification, and
arbitrary macro-generated modules are not certified by this conservative check.

The root checker activates this repository-wide graph only when the inventory
contains the target-ownership policy BUILD boundary. This keeps partial policy
fixtures partial; deleting the boundary makes the Bazel checker target fail to
analyze rather than silently disabling the rule.

## Repository shape

Handwritten `.rs`, `.swift`, and `.bzl` files are capped at 40 per directory, with an
empty grandfather list; a directory above the cap must split into feature
subdirectories. First-party labels shared by more than one registry load from
`topology.bzl`. A production module's scope, its own file plus its transitive `include!`
targets, is capped at 1,200 lines; the Host lifecycle and Worker PTY
compositions are recorded in the checker until they become module trees.
Every family root is either a real Bazel package with its own crate or a
filegroup-only aggregator over child packages, never both by accident.

## `include!` composition

`mod` is the unit of code organization. Production `include!` is limited to
generated contract files and test-target wiring from the same Bazel package;
any other composition site must already appear as an exact `(source, target)`
pair in `include_composition_baseline_runtime.bzl` or
`include_composition_baseline_platform.bzl`. Arguments must be a literal path:
dynamic forms such as `concat!(env!(...))` fail instead of evading review.
Paths are resolved before classification, so a `..` component cannot smuggle a
site through the test-directory exemption, and a parenthesized, bracketed, or
braced invocation is recognized identically. Every Rust file is scanned, so a
test file cannot compose production code through a nested include chain;
test-to-test wiring is admitted only inside the same package. The module budget
follows the same transitive graph. The baseline is a ratchet:
converting a site to a module removes its entry, and a new site fails the gate
instead of extending the list. Stale entries also fail, so the baseline cannot
drift from the sources. One strict Starlark reader parses both composition and
dependency-usage baselines, including multi-line tuples and comments, and a
malformed assignment fails instead of silently yielding no entries.

## Runtime facade

`//core:yolo_runtime` is the runtime composition facade. Production role
entries and test targets may use it; clients that only need Host/Worker DTOs
should prefer the owning protocol crate once extracted. Facade use is allowed
when it is explicit, and there is no blanket ban. The composition-root contract
(`checks/entrypoint`) requires exactly one production executable, reviews core
BUILD callables, and restricts the root's sources and dependencies. The
repository policy parses
`RUNTIME_COMPONENT_DEPS` structurally, resolves its named constants, and
requires every target name to occur as a whole identifier in `core/src`; a
malformed list or an unresolved name fails instead of skipping the check.

## Transport family

`core/crates/transport` holds wire primitives and runtime clients.
`client`, `relay`, `remote`, and `remote/broker` are clients, not primitives:
they consume Host/Worker DTOs through `yolo_runtime_protocol` (or the runtime
facade until narrowed) and must never be depended on by the engine. They stay
under the transport family for now; no separate runtime-client family is
introduced. The exact-edge test
`runtime_does_not_depend_on_transport_clients` enforces the direction.

## Hub boundaries

`hub_roles.bzl` classifies Hub targets as public models, clients, role
entries, or runtime; `hub_boundaries.bzl` derives the queries. The
`hub/policy` crate parses the four lists, resolving topology constants, and
the BUILD graph: every production Hub target must appear, a package may use at
most one of public, client, and runtime (an entry may accompany it), every
listed label must still name a declared target, a classified package must
still have a BUILD file and a production target (schema-only capnp packages
remain valid), and commented labels classify nothing. Public models may reach
only exact public-model packages; clients may not reach runtime internals; a
package-wide query starts from every Hub public/client/runtime target, so an
entry that is also a client is still checked, and asserts that no Hub library
or client depends upward on `hub/cli`, `hub/attach/cli`, or `hub/file/helper`. Adding a Hub package
therefore requires an explicit role decision instead of silently inheriting
either surface.

## Aggregate dependency usage

A production target whose `deps` come from a shared list can carry labels it
never uses. `target/dependency/usage` resolves those lists through the parsed
BUILD graph, maps each label to its `crate_name`, and fails when no source of
that target references the crate. The 39 sites found at introduction are
recorded in `dependency_usage_baseline.bzl`; a new unused label fails, and a
stale entry fails after the dependency is used or the list is split. The
composition root and the `//core:yolo_runtime` facade are exempt because their
dependencies are the exact role-entry contract checked separately. Resolution
follows local assignments and `load()`-imported lists, including constants in
the imported file. It is conservative: an unresolved dependency or source
expression is skipped, rows recorded for that target stay live instead of
becoming false stale entries, and any textual occurrence of the crate name
counts as use.

## Protocol registries

The Hub RPC keeps one tagged `Command`/`Reply` wire enum and its method,
access, origin, and reply-validation tables. Payload DTOs stay in their
domain protocol crates. Splitting the enum or its exhaustive matches by
domain would fragment the wire invariant without removing coupling; a generic
operation registry is not justified.

## Existing policy families

Mechanisms (`foundation`, `syntax`, `contracts`, `graph`, `test_support`) stay at the top level; policies live under `checks/`.
Process parsing is layered through `process_signal_catalog`, `process_signal_path_analysis`,
`process_signal_foundation`, `process_signal_child_policy`, `process_signal_retained_policy`,
and the thin `process_signal_policy` facade. The neutral `graph` crate owns the BUILD/module
graph and generic `build_constants`; process-signal policies consume it.
Native safety is divided into process, PTY, and boundary policy crates behind
`runtime_safety_policy`. Claude generation capability has its own
`checks/claude` crate. Repository checks are divided into Codex hook,
contract, inactive-E2E, literal, and UI policy crates behind
`repository_policy`. The root target owns only repository loading, overlay,
workspace location, and orchestration.

The remaining policy crates and root orchestrator cover repository layout,
retired toolchains and product identities, file length and test placement,
Bazel and Cargo boundaries, E2E focus isolation, process creation and signal
authority, PTY custody, socket and descriptor contracts, release-gate
composition, schema versions, marketing-version
identity between Bazel and the release contracts, composition-root entrypoint
contracts, and the macOS deployment floor. Behavioral parsers live in independently testable crates;
the root library is the traversal and policy orchestration facade.

## Verification Limits

The budgets above prevent unbounded build-time crates; they do not prove semantic
cohesion or finish decomposition of production Host, Worker, Process, and Paths
components. Other policy families protect explicit contracts, not every possible
runtime behavior. New safety contracts require explicit rules and adversarial tests.

The Rust coverage gate measures instrumented production code under `core`.
Build-system tool tests run in the unit suite, but their Rust implementation
coverage is not included in the product's greater-than-95-percent threshold.
Architecture conformance does not substitute for measured coverage, runtime tests,
packaged E2E, or release acceptance.
