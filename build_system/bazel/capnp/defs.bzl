"""Rust schema crates generated exclusively within Bazel actions."""

load("@rules_rust//rust:defs.bzl", "rust_library")
load(":generation.bzl", "capnp_rust_sources")

def capnp_rust_library(name, srcs, crate_name, crate_root = None, deps = [], **kwargs):
    """Generate one crate's mutually importing schemas, with an optional authored crate root.

    An authored src/lib.rs uses include!("generated/<schema>_capnp.rs") inside
    pub mod <schema>_capnp. Generated sources are compile_data, never checked in.
    Schema files must live beside their BUILD.bazel. The compiler cannot read
    system import directories; every imported project schema belongs in srcs.
    """
    generated = name + "_generated"
    capnp_rust_sources(name = generated, srcs = srcs, testonly = kwargs.get("testonly", False))
    if crate_root == None:
        root_target = name + "_crate_root"
        native.filegroup(name = root_target, srcs = [":" + generated], output_group = "crate_root",
                         testonly = kwargs.get("testonly", False))
        crate_root = ":" + root_target
    rust_library(
        name = name,
        srcs = [crate_root],
        crate_root = crate_root,
        compile_data = [":" + generated],
        crate_name = crate_name,
        edition = "2024",
        deps = deps + ["@crates//:capnp"],
        **kwargs
    )
