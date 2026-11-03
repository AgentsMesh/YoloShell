# Darwin coverage layout

Continuous LLVM profiling replaces complete memory pages with `MAP_FIXED` file
mappings. Aligning the start of the counters is insufficient: its final page
must not contain unrelated globals, including sections introduced by an optimizer
or another linked object.

`profile_link_flags` moves `__llvm_prf_cnts` and `__llvm_prf_bits` into their own
reserved writable segments, `__YOLO_PRFC` and `__YOLO_PRFB`, and aligns their
sections to 16 KiB. The `__DATA,__llvm_prf_data` start remains aligned because
compiler-rt checks it too. Mach-O linker boundary symbols follow the section
renaming; coverage mapping and counter identities are unchanged. Non-Darwin
flags and environment are unchanged. No LLVM pass is moved.

The layout tests inspect actual thin 64-bit Mach-O files without invoking
`otool`. They reject malformed input and prove that each dedicated segment has
only its expected section, that its mapped page interval is contained in a
writable segment, and that no other section or segment occupies those pages.
This is a geometric invariant, independent of adjacent section names.

An O3 fixture includes a nonstandard `__DATA` section with an initialized atomic
marker. Its normal and SIGKILL continuous profiles must preserve the marker and
match the ordinary profile's observed function counts. A second O3 target uses
the actual Bazel coverage pipeline instead of forcing instrumentation.

A separate Clang C fixture uses `-fprofile-instr-generate -fcoverage-mapping
-fcoverage-mcdc` and all eight inputs of `a && (b || c)` to produce a real,
nonempty MC/DC bitmap. Rust 1.93's `coverage-options=condition` does not produce
MC/DC bitmaps. Both natural-exit and SIGKILL continuous profiles must retain the
same MC/DC records and covered independence pairs as the ordinary profile.
The fixture uses the same dedicated linker segments; the application's
branch-coverage configuration and coverage gate are unchanged.

Targets:

- `:unit_tests`: evaluated Starlark flags and non-Darwin behavior.
- `:layout_unit_tests`: malformed Mach-O and adversarial layout fixtures.
- `:layout_integration_tests`: actual O3, unknown-section and MC/DC binaries.
- `:integration_tests`: the existing lifecycle and counter-accounting probes.
