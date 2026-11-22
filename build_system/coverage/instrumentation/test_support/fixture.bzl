"""Toolchain metadata and evaluated coverage settings for the Rust tests."""

load("//build_system/coverage/instrumentation:defs.bzl", "MACOS", "profile_environment", "profile_link_flags")

_TOOLCHAIN = "@rules_rust//rust:toolchain_type"
_NON_DARWIN = ["linux", "windows", "freebsd"]

def _runfile(ctx, file):
    if file.short_path.startswith("../"):
        return file.short_path[3:]
    return ctx.workspace_name + "/" + file.short_path

def _fixture_impl(ctx):
    toolchain = ctx.toolchains[_TOOLCHAIN]
    output = ctx.actions.declare_file(ctx.label.name + ".json")
    tools = [toolchain.llvm_cov, toolchain.llvm_profdata]
    ctx.actions.write(output, json.encode({
        "llvm_cov": _runfile(ctx, tools[0]),
        "llvm_profdata": _runfile(ctx, tools[1]),
        "coverage_enabled": ctx.configuration.coverage_enabled,
        "macos_flags": profile_link_flags(MACOS),
        "direct_macos_flags": profile_link_flags(MACOS, direct_linker = True),
        "macos_environment": profile_environment(MACOS),
        "other_flags": {os: profile_link_flags(os) for os in _NON_DARWIN},
        "other_environments": {os: profile_environment(os) for os in _NON_DARWIN},
    }))
    return [DefaultInfo(files = depset([output]), runfiles = ctx.runfiles(files = [output] + tools + toolchain.llvm_lib))]

coverage_fixture = rule(implementation = _fixture_impl, toolchains = [_TOOLCHAIN])
