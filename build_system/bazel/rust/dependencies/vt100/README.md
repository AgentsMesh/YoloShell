# vt100 0.16.2 cursor-report patch

`cursor_position.patch` adds `Screen::report_cursor_position() -> (u16, u16)`.
Its values are **one-based CPR coordinates**, ready for `CSI row ; col R`.
The row uses DEC origin mode and the active grid's scrolling margin. A
wrap-pending cursor reports the last physical column without consuming the
pending wrap. The existing zero-based `cursor_position()` API is unchanged.

The Bazel `crate.annotation` in `MODULE.bazel` applies the patch to the exact
Cargo-pinned upstream version. Never modify extracted external repositories.
The existing checksum in `Cargo.lock` continues to authenticate the original
crate archive; the repository-owned patch describes the complete modification.

`//build_system/bazel/rust/dependencies/vt100:cursor_position_test` names the terminal
integration target, which exercises the patched dependency alongside its actual
product adapter. Tests include origin mode, margins, wrap, save/restore, alternate
grids and scrollback. The terminal query component provides additional
end-to-end request/reply tests. Cargo is used for dependency
metadata only; Bazel owns the patched product and test builds.
