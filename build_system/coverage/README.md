# Rust Coverage Scopes

The collector emits one integrity-checked source and instance inventory for all
instrumented production Rust in a report. The gate validates every inventory and
metric vector before selecting a scope, including sources outside that scope.
Dropping a tooling source from a shared inventory therefore also rejects the
default product gate.

## Product Gate

The existing single-report invocation is unchanged:

```sh
bazel run //build_system/coverage/gate:gate -- bazel-out/_coverage/_coverage_report.dat
```

It measures production Rust sources under `core/src`, `core/crates` and `plugins`.
The Core reporting scope includes the delivered client plugins; directory moves
do not remove their sources from the coverage denominator.
`--scope=core` explicitly selects the same scope. Line, function, region, and
branch coverage must each be strictly greater than 95 percent. Tooling coverage
never contributes to this denominator or numerator.

## Tooling Gate

Collect build-system Rust coverage separately with:

```sh
bazel coverage --config=rust_tooling \
  //build_system/architecture:tests //build_system/release:tests \
  //build_system/coverage:tests //build_system/platform/macos/inactive_cleanup:tests
bazel run //build_system/coverage/gate:gate -- \
  --scope=tooling bazel-out/_coverage/_coverage_report.dat
```

The tooling gate prints the same four measured metrics and fails if any is at
or below 95 percent. An absent scope or zero metric denominator is an error,
not a successful empty measurement. A failed threshold still prints the actual
totals; a malformed inventory fails before printing coverage percentages.

Tooling sources must be Rust files beneath a `build_system` package's `src`
directory. Unit tests, integration tests, fixtures, generated source directories,
and `test_support` packages are excluded. Production policy modules such as
`src/test_ownership.rs` remain included despite their names.

Both scopes can be collected in one invocation by using the instrumentation
configuration `rust_tooling` and selecting both core and tooling test suites.
Run the gate once for each scope against that report. Source and instance
inventories cover the entire report in both invocations; source filtering is
never used to bypass malformed or missing coverage evidence.

The tooling collection configuration retains both production source roots and
the repository's `integration_tests` source manifest entries.
Some tooling tests link the core runtime; filtering core out of Bazel's source
manifest removes members of their LLVM source inventories during merging.
Collection therefore preserves these transitive records, while the selected gate
still measures only its own scope.

The default release producer still uses `--config=rust`, whose filter includes
`core`, `plugins` and `integration_tests`. Test sources provide execution evidence without
entering the production denominator. Tooling measurement does not weaken or
change the product release gate, and does not claim coverage for Swift tools.

## Collector Completion

Production function, region, and branch execution counts must be below LLVM's
signed saturation boundary. LLVM JSON can clamp a wrapped negative counter
expression to `i64::MAX`; that value cannot attest a covered execution. The
collector rejects saturated or larger counts with the source and function name,
leaves its incomplete marker intact, and publishes no report. It does not clamp
the bad value, count it as a hit, or remove the source from the denominator.
The existing production-source scope is unchanged; test-only functions do not
become production instances because their counts are invalid.

The collector owns a `TEST_PREMATURE_EXIT_FILE` marker only when it creates that
file itself. It removes the marker after successful report publication and only
while the retained file identity and metadata still match. Existing failure
markers, replacements, and failed collection paths are never cleared. This makes
Bazel reject collector failures even when its coverage wrapper reports exit zero.

The runner probe is healthy by default. Each deliberate failure below must return
a failing Bazel status, even though the probe's Rust test itself passes:

```sh
bazel coverage --config=rust_tooling //build_system/coverage/runner:unit_tests \
  --nocache_test_results --test_env=YOLO_COVERAGE_FAILURE_PROBE=1
bazel coverage --config=rust_tooling //build_system/coverage/runner:unit_tests \
  --nocache_test_results --test_env=YOLO_COVERAGE_FAILURE_PROBE=preserve-existing-marker
```

`profile_test_support` is test-only. Its exact hostname, PID, and module matcher
may be called only after a deliberately terminated fixture has been reaped or
its retained process identity is proven gone. Unknown templates, PID-prefix
lookalikes, symlinks, and unrelated corrupt profiles are not discarded. Normal
completion preserves coverage; this helper is not a collector corruption filter.

## Declaration-Only Targets

The release contracts, evidence model, maintenance contract, and session timing
packages contain constants, types, traits, and derived implementations, but LLVM
emits no production source mappings for their tests. These tests still run with
coverage enabled. The collector accepts that
specific absence only when Bazel's entire production source manifest equals a
reviewed source set in `source_contract/src/reviewed.rs`, every source SHA-256 matches,
and LLVM records an executed, manifest-owned test function and its file summary.

Changed source bytes, unknown or omitted manifest members, missing test mappings,
and production functions without summaries remain errors. New instrumented
production functions take the normal collection path. When declaration-only
source text changes, its fingerprint needs explicit review; the collector never
learns a new fingerprint automatically. No synthetic production hits or zero
denominators are added, and the scoped percentage gates remain unchanged.

Targets containing only normalized `unit_tests`, `integration_tests` and
`test_support` Rust sources can also complete collection without production
mappings. Admission requires a manifest-owned executed test with matching LLVM
function/file evidence and rejects unknown sources, undeclared mappings or any
missing production member. Their real test records are retained without creating
production metric tokens. Empty percentage scopes still fail the gate.

LLVM's external-source filter removes dependency file summaries while retaining
their function mappings in JSON. Test-only admission applies that established
exclusion only to normalized external dependency function paths; all file summaries
and every repository function path still require exact manifest ownership.
Dependency functions cannot witness an executed test, and mixed mappings cannot
hide an undeclared repository source. Rejection diagnostics identify the offending
raw and normalized source paths.

Repository-level `integration_tests` fixtures may accompany explicit nested
`unit_tests` or `integration_tests` sources. Fixture helpers alone cannot witness
test execution. Both Rust coverage configurations retain this root in Bazel's
source manifest; missing source entries cannot be inferred from LLVM mappings.
Root lookalikes, traversal, empty path components and undeclared mappings fail
admission. The production source classifier and percentage gates are unchanged.

## Darwin Continuous Profiles

Coverage-enabled macOS Rust binaries isolate counters and MC/DC bitmaps in two
dedicated Mach-O segments, align profile sections to 16 KiB, and use Bazel's
continuous profile mode. Section-start alignment alone is insufficient: an
optimized binary can place unrelated data in the last mapped counter page.
Forced-exit profiles remain available rather than being removed by test cleanup.
Non-continuous matching and cleanup are unchanged; corruption still fails collection. See the repository coverage notes for the runtime/layout contract, red/green
probes and remaining full-Core acceptance.

The explicitly selected `rust_coverage_direct` configuration imports the pinned
experimental compiler described in [compiler/BAZEL.md](compiler/BAZEL.md). It uses
physical counters across calls and continuations to test the known interrupted
call accounting defect. It does not lower thresholds or change source eligibility;
the default compiler remains unchanged until producer and complete-suite
acceptance establish the new producer's correctness.

The release, architecture and inactive-cleanup CLI integration targets execute
their actual binaries and declare them as `coverage_objects`. This preserves
their `main.rs` mappings and the inherited LLVM profile destination across child
working-directory changes. Help and controlled rejection scenarios do not run
publication, signing or user-service cleanup. Selecting only library tests does
not establish coverage of these process entry points.

## Generated-input Source Staging

Generated `compile_data` can make rules_rust stage handwritten sources beneath
`bazel-out/<configuration>/bin`. Packages using such inputs must preserve each
handwritten source's canonical identity with `canonical_source_path_flags` from
`build_system/bazel/rust/source_paths.bzl`, using the same source list as the Rust rule.
Only exact declared source files are remapped; generated neighbours are not.

Before production filtering, the collector examines LLVM file summaries,
function filenames and LCOV source records for these staged paths. If the
canonical handwritten source is declared in Bazel's bounded source manifest,
an unremapped alias fails collection. Zero execution counts do not waive the
check, and other valid dependency mappings cannot conceal the missing component.
The failure leaves the incomplete marker and publishes no partial report.

This check rejects missing mapping evidence rather than changing source scope,
accepting generated files as production, or modifying metric counts. Normal
canonical reports do not require an additional manifest read. A full report
still needs all selected tests and the unchanged Core/tooling percentage gates.
