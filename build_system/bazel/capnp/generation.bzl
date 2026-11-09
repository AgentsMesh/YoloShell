"""Sandboxed schema generation using pinned C++ and Rust compiler tools."""

def _generate(ctx):
    if not ctx.files.srcs:
        fail("At least one Cap'n Proto schema is required.")
    package = ctx.label.package
    names = []
    outputs = []
    for source in ctx.files.srcs:
        if source.dirname != package:
            fail("Schemas must be declared beside BUILD.bazel; use local imports within the schema crate.")
        stem = source.basename[:-len(".capnp")].replace("-", "_")
        if stem in names:
            fail("Duplicate generated schema module: " + stem)
        names.append(stem)
        outputs.append(ctx.actions.declare_file("src/generated/" + stem + "_capnp.rs"))
    root = ctx.actions.declare_file("src/generated/lib.rs")
    ctx.actions.write(root, "\n".join(["pub mod " + name + "_capnp;" for name in names]) + "\n")
    args = ctx.actions.args()
    args.add("compile")
    args.add("--no-standard-import")
    args.add("--src-prefix=" + package)
    args.add("-I" + package)
    args.add("-o" + ctx.executable._generator.path + ":" + root.dirname)
    args.add_all(ctx.files.srcs)
    ctx.actions.run(
        executable = ctx.executable._compiler,
        arguments = [args],
        inputs = ctx.files.srcs,
        tools = [ctx.attr._generator[DefaultInfo].files_to_run],
        outputs = outputs,
        mnemonic = "CapnpRustGenerate",
        progress_message = "Generating Cap'n Proto Rust schemas %{label}",
    )
    return [
        DefaultInfo(files = depset(outputs + [root])),
        OutputGroupInfo(crate_root = depset([root])),
    ]

capnp_rust_sources = rule(
    implementation = _generate,
    attrs = {
        "srcs": attr.label_list(allow_files = [".capnp"], mandatory = True),
        "_compiler": attr.label(default = "@capnp-cpp//src/capnp:capnp_tool", executable = True, cfg = "exec"),
        "_generator": attr.label(default = "//build_system/bazel/capnp/generator:capnpc-rust", executable = True, cfg = "exec"),
    },
)
