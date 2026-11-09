# Explicit local Bazel coverage compiler

The official Rust 1.93.1 toolchain remains the default. MODULE only declares two lazy repositories;
neither is registered globally. `--config=rust_coverage_direct` adds the local toolchain through
`--extra_toolchains`. It applies to explicit build/test commands as well as coverage; coverage
also retains the existing `rust_tooling` source scope and branch instrumentation.

The local import currently supports native `aarch64-apple-darwin`. It uses stage1 rustc, rustdoc,
driver libraries and the complete matching std build, plus llvm-cov/llvm-profdata and their
dynamic libraries from the same local LLVM build. The native C/C++ toolchain supplies the linker.
No stock Rust rlibs, rustdoc or LLVM tools are substituted. Missing artifacts fail the import.

## Complete build prerequisite

From the prepared Rust source directory:

```sh
./x build --stage 1 library src/tools/rustdoc \
  --config ../../build_system/coverage/compiler/bootstrap.macos.toml
```

Both targets must be requested together. A later rustdoc-only bootstrap can remove std from
stage1. Repeating the complete command reassembles it from cached products. Repository rules
never invoke bootstrap, compile a probe or repair an incomplete sysroot.

## Import receipt

`YOLO_COVERAGE_RUSTC_ROOT` is the only local path selector. The named config explicitly sets it
to `.build/coverage-rustc-1.93.1`. A relative value resolves from the main checkout root, derived
from the repository-owned pin label. Canonical resolution must remain under that checkout's
`.build`; no user-specific absolute path is stored in source.

Generate or refresh the content-bound import receipt without building a target:

```sh
bazel fetch \
  --repo_env=YOLO_COVERAGE_RUSTC_ROOT=.build/coverage-rustc-1.93.1 \
  @yolo_coverage_rustc_build//:receipt
```

The repository produces `@yolo_coverage_rustc_build//:build-receipt.json` with this exact schema:

| Field | Evidence |
| --- | --- |
| `schema_version` | Format version 1 |
| `source_root` | Canonical path relative to the main checkout |
| `source_commit`, `submodule_commits` | Actual Git identities, checked against pin.toml |
| `brand`, `host`, `llvm_version` | Pinned expected identities |
| `rustc_verbose_version`, `rustdoc_version`, `llvm_tool_versions` | Actual executable output |
| `patch_sha256` | Digest for each patch, checked against the exact source diff |
| `metadata_sha256` | Pin, bootstrap config and Cargo std-build record digests |
| `artifact_sha256` | Logical-path-to-SHA-256 map for every imported tool/library |

The std-build record must identify the currently executed branded rustc. Every imported std
artifact must also match its corresponding `stage1-std/<target>/release/deps` artifact by digest.
The importer independently repeats the observation and requires exact equality with the receipt.
Unexpected fields or changed inventories cannot match. Source, tool and std inputs are watched,
so their changes invalidate local repository observations; an existing receipt is not blindly
trusted.

The compiler's real `commit-hash: unknown` remains in its verbose version. `source_commit` is
separately observed from the pinned source tree, never substituted into rustc output. This receipt
binds locally observed source and products with their std-build metadata; it is not a signed or
independently reproduced proof of the compilation process. Successful mutable log text is not
used as admission authority.

## Required pipeline checks

Use a fresh Bazel output base for this producer, preserving earlier profiles and logs:

```sh
bazel --output_base="$PWD/.build/bazel-direct-coverage" test \
  --config=rust_coverage_direct --nocache_test_results \
  //build_system/coverage/instrumentation:integration_tests

bazel --output_base="$PWD/.build/bazel-direct-coverage" coverage \
  --config=rust_coverage_direct --nocache_test_results \
  //build_system/coverage/instrumentation:integration_tests
```

Run the complete instrumentation target. In particular, do not filter away the original
interrupted-result failure or the pipeline exit/SIGKILL cases. Its fixture receives the selected
toolchain's actual LLVM tools through ToolchainInfo, so coverage mode exercises Bazel's real
producer and collector path. Both target and exec Rust compilations use direct counters under
this explicit toolchain; without instrumentation the option has no effect.

Attach the emitted build receipt to experiment results. Keep new raw profiles separate from
stock and earlier experimental layouts, even when their source names coincide. The compiler
brand participates in crate identity, and actual tool/std digests become Bazel action inputs.
Do not change coverage thresholds, exclude production mappings or reuse old reports as proof
that this configuration passed. The complete instrumentation and compiler probe targets passed
through real Bazel coverage in `.build/direct-coverage-producer-and-hub-regression.log`.
That selected batch also preserves a full source/instance inventory in
`.build/direct-coverage-producer-and-hub.lcov`; it is not the complete Core/tooling suite.
Real Host/Worker failure coverage and final-source percentage acceptance remain required.
