"""Explicit local coverage compiler repositories; never build or register a toolchain."""

load(":repository_common.bzl", "RECEIPT_FILE", "ROOT_ENV")
load(":repository_receipt.bzl", "observe_build")

def _receipt_impl(ctx):
    observation = observe_build(ctx)
    ctx.file(RECEIPT_FILE, json.encode_indent(observation.receipt) + "\n")
    ctx.file("BUILD.bazel", 'exports_files(["{}"], visibility = ["//visibility:public"])\nfilegroup(name = "receipt", srcs = ["{}"], visibility = ["//visibility:public"])\n'.format(RECEIPT_FILE, RECEIPT_FILE))

def _toolchain_impl(ctx):
    recorded = json.decode(ctx.read(ctx.attr.receipt))
    observation = observe_build(ctx)
    if recorded != observation.receipt:
        fail("coverage compiler build receipt no longer matches source, tools or std artifacts")
    for logical, path in observation.files.items():
        ctx.symlink(path, logical)
    ctx.file(RECEIPT_FILE, json.encode_indent(recorded) + "\n")
    ctx.template("BUILD.bazel", ctx.attr.build_template, {"{HOST}": recorded["host"]})

def _attrs():
    return {
        "pin": attr.label(default = Label(":pin.toml"), allow_single_file = True),
        "bootstrap": attr.label(default = Label(":bootstrap.macos.toml"), allow_single_file = True),
        "patches": attr.label_list(default = [Label(":0001-direct-counter-mode.patch"), Label(":0002-physical-bcb-counters.patch")], allow_files = True),
    }

coverage_rustc_receipt = repository_rule(
    implementation = _receipt_impl,
    attrs = _attrs(),
    environ = [ROOT_ENV],
    local = True,
)

_TOOLCHAIN_ATTRS = _attrs()
_TOOLCHAIN_ATTRS.update({
    "receipt": attr.label(mandatory = True, allow_single_file = True),
    "build_template": attr.label(default = Label(":toolchain.BUILD.bazel.tpl"), allow_single_file = True),
})

coverage_rustc_toolchain = repository_rule(
    implementation = _toolchain_impl,
    attrs = _TOOLCHAIN_ATTRS,
    environ = [ROOT_ENV],
    local = True,
)
