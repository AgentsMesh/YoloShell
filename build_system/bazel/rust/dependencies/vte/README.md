# vte 0.15.0 bounded OSC patch

`osc_buffer.patch` bounds retained Operating System Command payloads in both
the TUI's query detector and the `vt100` screen parser. Both use the same exact
patched dependency. Upstream's `std` implementation otherwise grows a `Vec`
across arbitrarily many terminal output frames until an OSC terminator arrives.

`vte::MAX_OSC_RAW_BYTES` is 65,536 bytes. This per-parser limit accommodates
ordinary titles, directory notifications and palette commands while remaining
independent of output frame boundaries. The protocol permits 1 MiB output
chunks, so its frame bound alone cannot bound a command spanning many chunks.
The raw payload budget excludes semicolon separators, whose index storage has
its own fixed upstream limit. A smaller `no_std` parser buffer remains a
stricter bound.

An over-limit command is discarded entirely: no truncated title, clipboard or
other OSC callback is dispatched. The parser remains in OSC state and consumes
bytes until the normal upstream terminator or cancellation. Overflow never
resets the parser to Ground, so ordinary payload cannot become terminal input.
Subsequent complete commands work normally. Buffers retain at most the stated
bounded capacity for reuse; no unbounded payload is retained across frames.

The Bazel annotation applies only to Cargo-pinned vte 0.15.0. Cargo's checksum
continues to authenticate the original archive; this repository owns the patch.
Never modify extracted external repositories. Cargo supplies dependency metadata;
Bazel owns patched builds and tests.

`//build_system/bazel/rust/dependencies/vte:osc_buffer_test` aliases the real terminal
query integration target. Its public parser regressions cover payloads before,
at and over the bound, fragmented overflow, silent oversized OSC callbacks,
payload that resembles CSI, and recovery of later OSC and CSI commands.
