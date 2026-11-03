"""Generates the Rust SCM_RIGHTS descriptor ceiling contract."""

_MAXIMUM_DESCRIPTOR_COUNT = 512
_RUST_OUTPUT_PATH = "src/generated/%{name}.rs"

def _rust_contract_impl(ctx):
    content = (
        "pub(super) const MAX_RECEIVED_DESCRIPTORS: usize = %s;\n" %
        _MAXIMUM_DESCRIPTOR_COUNT
    )
    ctx.actions.write(ctx.outputs.out, content)

_rust_contract = rule(
    implementation = _rust_contract_impl,
    outputs = {"out": _RUST_OUTPUT_PATH},
)

def scm_rights_descriptor_rust_contract(name):
    _rust_contract(name = name)
