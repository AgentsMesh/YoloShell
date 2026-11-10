# Experimental direct coverage counters

This directory prepares a pinned Rust coverage producer experiment. The default Bazel toolchain
and collection policy remain unchanged. `BAZEL.md` describes its explicit local import and build
receipt. Focused producer probes are required before complete repository coverage acceptance.

`pin.toml` identifies Rust 1.93.1 commit
`01f6ddf7588f42ae2d7eb0a2f21d44e8e96674cf`, its bootstrap compiler, and patch order.
Downloaded source belongs in `.build/coverage-rustc-1.93.1` within this checkout and is ignored
by the product repository. The pinned repository supplies all submodule commit identities.

## Producer change

`0001-direct-counter-mode.patch` adds `-Zcoverage-direct-counters`, a tracked boolean that
defaults to false. When enabled, coverage graph construction does not combine Call, Drop,
Assert or InlineAsm terminators with their continuations. Yield already has this property.
Default-mode graph unit tests explicitly retain the old mode.

`0002-physical-bcb-counters.patch` makes the coverage query allocate physical counters for
all surviving BCBs referenced by code or true/false branch mappings. It bypasses balanced-flow
counter inference. Optimized-away BCB mappings remain present with zero hits. Existing virtual
counters, atomic LLVM increments, mapping emission, and profile formats are reused.

The patches do not change coverage eligibility, source exclusions, macro policy or branch
selection. Splitting previously conflated continuations can change region geometry and increase
region counts. Review that difference as a producer correction; do not lower thresholds or
exclude production files/functions/branches to make the new report pass.

## Prepare or audit the source

Run these commands from the product checkout root. Clone only when the destination is absent;
an existing prepared checkout must be audited without resetting or overwriting it.

```sh
git clone --filter=blob:none --depth 1 --branch 1.93.1 \
  https://github.com/rust-lang/rust.git .build/coverage-rustc-1.93.1
git -C .build/coverage-rustc-1.93.1 rev-parse HEAD
git -C .build/coverage-rustc-1.93.1 submodule update --init --depth 1 --filter=blob:none \
  -- library/backtrace src/llvm-project
git -C .build/coverage-rustc-1.93.1 apply --check \
  ../../build_system/coverage/compiler/0001-direct-counter-mode.patch \
  ../../build_system/coverage/compiler/0002-physical-bcb-counters.patch
git -C .build/coverage-rustc-1.93.1 apply \
  ../../build_system/coverage/compiler/0001-direct-counter-mode.patch \
  ../../build_system/coverage/compiler/0002-physical-bcb-counters.patch
```

The prepared checkout already has both patches. Audit that state without modifying it:

```sh
git -C .build/coverage-rustc-1.93.1 diff --check
git -C .build/coverage-rustc-1.93.1 apply --reverse --check \
  ../../build_system/coverage/compiler/0002-physical-bcb-counters.patch \
  ../../build_system/coverage/compiler/0001-direct-counter-mode.patch
git -C .build/coverage-rustc-1.93.1 submodule status library/backtrace src/llvm-project
```

Require the exact pinned HEAD and no modified submodule revisions. A patch check is not a
compiler build or a correctness test.

## Build stage1 when authorized

From this product checkout root:

```sh
cd .build/coverage-rustc-1.93.1
./x build --stage 1 library src/tools/rustdoc \
  --config ../../build_system/coverage/compiler/bootstrap.macos.toml
```

The resulting compiler and sysroot are at
`.build/coverage-rustc-1.93.1/build/aarch64-apple-darwin/stage1` relative to the product checkout.
Bootstrap downloads the pinned stage0 and builds the pinned LLVM source for the native AArch64
target; profiler support is enabled and compiler-rt sources are prepared. The pinned CI LLVM
artifact returned HTTP 404 in the first bootstrap attempt, recorded in
`.build/coverage-rustc-stage1-build.log`. The source build uses four jobs and does not request
stage2. Bootstrap may fetch additional dependencies. The matching standard library is built
for the same native target; Linux coverage needs its own matching compiler build.

Build library and rustdoc together. A rustdoc-only bootstrap can reassemble stage1 without its
std rlibs; repeating the complete command restores the matching products from the build cache.

`build.description = "yolo-direct-coverage-v1"` separates this compiler's identity and mangled
crate symbols from stock Rust. Keep the custom sysroot together with its compiler. Do not point
it at the stock sysroot or reuse stock instrumented rlibs.

## Measurement isolation and acceptance

See `PROBES.md` for the minimum execution witnesses. Compile the same probe source with mode
off and on under this compiler, using separate output and profile directories for each. The
experimental mode requires all three options:

```text
-C instrument-coverage
-Zcoverage-options=branch
-Zcoverage-direct-counters
```

Preserve Darwin's separate counter/bitmap segments, 16 KiB alignment and the `%c` profile
convention for interrupted processes. Section-start alignment alone did not protect optimized
globals in the final mapped page. Use the matching LLVM 21 tools. Keep old profiles as evidence; collect
new profiles in a fresh experiment directory whose metadata identifies the compiler commit,
patch digest, compiler brand, flags and object digest. Never merge the old and new layouts.

The native stage1 build, same-source interrupted-call witnesses and twelve O0/O3 profile
comparisons have completed. The Bazel compiler integration test also passed six cancellation
and panicking-drop variants, including ThinLTO and FatLTO. Its sources and assertions are
committed with this experiment; see `PROBES.md`. Instrumentation tests pass through the local
toolchain, while complete coverage-mode collection and real Host/Worker acceptance remain
required. No full repository coverage percentage is established by these producer probes.
