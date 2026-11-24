pub const CAPNP_DEFINITIONS: &str = r#"
load("@rules_rust//rust:defs.bzl", "rust_library")
load(":generation.bzl", "capnp_rust_sources")
def capnp_rust_library(name, srcs):
    capnp_rust_sources(name = name, srcs = srcs)
    native.filegroup(name = name + "_root", srcs = [])
    rust_library(name = name, srcs = [])
"#;
pub const CAPNP_GENERATION: &str = r#"
def _generate(ctx):
    count = len(ctx.files.srcs)
    path = ctx.label.name.replace("-", "_")
    args = ctx.actions.args()
    args.add(path)
    args.add_all(ctx.files.srcs)
    ctx.actions.run(arguments = [args])
    return [OutputGroupInfo(crate_root = depset([]))]
capnp_rust_sources = rule(implementation = _generate)
"#;
pub const UNREVIEWED_BINARY_MACRO: &str = r#"
load("@rules_rust//rust:defs.bzl", "rust_binary")
def extra_binary(name):
    rust_binary(name=name, crate_root="src/main.rs", srcs=["src/main.rs"])
"#;
