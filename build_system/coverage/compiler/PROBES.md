# Direct-counter producer acceptance

Use the original sources, with mode off and on, in separate build/profile directories.
The off build is the control. Keep the same target, optimization, feature, source eligibility
and branch coverage settings. Read every raw profile; do not discard interrupted instances,
clamp exported counts or repair reports. Baseline failures remain evidence.

## Required witnesses

| Case | Required independent result |
| --- | --- |
| First callee blocks, caller is killed | One caller entry; zero subsequent loop iterations and returns |
| Kill while inside the first loop iteration | Exactly the reached body/branch; no later iteration or return |
| Ordinary Err and early return | Exact taken return path; unreached continuation stays zero |
| Catch a panic from the first or second call | Original payload preserved; later success/return spans stay zero |
| Accounting script, all cases | 48 entries and 30 normal returns; branch witnesses remain exact |
| Accounting script, panic-only | 18 entries and zero normal returns |
| main returns with five suspended threads | Six entries, one completed call/return; no five phantom successes |
| Four joined threads, then SIGKILL | Exactly 400,000 observed calls and zero cold-function calls |
| Pending future is dropped | No resume or post-await continuation hit |
| Destructor blocks or unwinds | No destructor-return continuation hit |
| Unreached code and function | Zero hits; the production mappings remain in the denominator |

Existing evidence sources in this workspace include:

- `build_system/coverage/instrumentation/test_support/profile_probe.rs`
- `build_system/coverage/compiler/test_support/cancellation_probe.rs`
- `.build/coverage_live_loop_probe.rs`
- `.build/coverage_producer_accounting_probe.rs`

The `.build` sources are diagnostic artifacts, not a committed test suite. Their exact bytes
and independent witnesses must be retained with any reproduction. New focused coverage tests
should be added to the patched compiler's upstream coverage tests before adopting the patch.

## Inspect the producer as well as the report

1. Dump instrumented MIR and LLVM IR. Confirm the call and its normal continuation have separate
   virtual/physical counters in direct mode, and the continuation increment is after the call.
2. Confirm code/branch terms use physical counters or genuine optimized-away zero mappings. The
   direct-mode expression table should be empty; this is a producer property, not report filtering.
3. Read raw physical counters and LLVM JSON/LCOV output. Match branch and normal-return counts to
   independent witnesses, including finite positive values. Absence of saturation is insufficient.
4. Compare source/function/branch inventories and explain changed region geometry. Reject dropped
   production paths, changed coverage eligibility or any use of exclusions to improve the ratio.
5. Repeat at optimization levels 0 and 3, with one and default codegen-unit counts. Include
   short-circuit conditions and let-chains so joined branches do not gain other paths' counts.
6. Keep compiler brand/patch/object provenance for every profile. Then validate the real Host and
   Worker failure targets before spending time on a new complete repository coverage run.

Same-source live/accounting probes and twelve O0/O3/codegen-unit profile comparisons passed
in `.build/direct-counter-probes.DOg958`. The committed `:integration_tests` target exercises
the cancellation/destructor witness at O0/O3 with default/single codegen units and O3
ThinLTO/FatLTO; all six passed in `.build/direct-coverage-cancellation-matrix.log`.
The zero continuation functions must remain mapped, and all exported counts must remain finite.
The live/accounting diagnostic sources still need durable automated regression coverage;
full Host/Worker and repository acceptance are not established by these results.

The strengthened cancellation target also binds unique markers in the original
fixture source to nonempty caller mappings through each mapping's file ID. It
requires four distinct entered functions/bodies once, both caller continuations
zero, and the destructor's true/false branch as 1/0. The most specific source
geometry selects the mapping; missing, equal or crossing minima fail. No line
maximum or callee count substitutes for a caller witness. All six variants passed
in `.build/direct-coverage-caller-witness-matrix.log`. Complete repository coverage
and durable live/accounting regression fixtures remain separate acceptance work.
