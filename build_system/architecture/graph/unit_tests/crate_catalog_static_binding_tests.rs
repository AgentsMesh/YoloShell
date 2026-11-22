use super::*;

#[test]
fn non_static_constant_forms_fail_closed() {
    for (relative, definition) in [
        (
            "build_system/bazel/rust/duplicate.bzl",
            "ROOT = \"src/a.rs\"\nROOT = \"src/b.rs\"\nrust_binary(crate_root = ROOT)",
        ),
        (
            "build_system/bazel/rust/expression.bzl",
            "ROOT = \"src/\" + \"main.rs\"\nrust_binary(crate_root = ROOT)",
        ),
        (
            "build_system/bazel/rust/incremental.bzl",
            "ROOT += \"src/main.rs\"\nrust_binary(crate_root = ROOT)",
        ),
        (
            "build_system/bazel/rust/macro.bzl",
            "def target(ROOT = \"src/clean.rs\"):\n    rust_binary(crate_root = ROOT)\ntarget(\"src/retained.rs\")",
        ),
    ] {
        let catalog = one(relative, definition);
        assert!(catalog.roots.is_empty(), "{relative}");
        assert_eq!(catalog.unresolved, set(&[relative]));
    }
}
