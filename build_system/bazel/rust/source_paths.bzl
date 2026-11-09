"""Preserve declared source identities when rules_rust stages generated inputs."""

_RUST_EXTENSION = ".rs"
_GENERATED_COMPONENT = "generated"
_SOURCE_PREFIX = "src/"
_BIN_ROOT = "$(BINDIR)"
# Make expansion unescapes this dollar; rules_rust's process wrapper then resolves
# the actual sandbox execroot. Target rustc_flags follow its default pwd remap.
_ABSOLUTE_ROOT = "$${pwd}/"

def canonical_source_path_flags(srcs):
    """Map only local handwritten files from the same list supplied to srcs.

    Use a native.glob source list, never generated labels or compile_data. Exact
    file mappings leave generated neighbours outside the production source set.
    """
    package = native.package_name()
    flags = []
    for source in srcs:
        if (not source.startswith(_SOURCE_PREFIX) or
            not source.endswith(_RUST_EXTENSION) or
            any([part in ["", ".", "..", _GENERATED_COMPONENT] for part in source.split("/")]) or
            ":" in source):
            fail("source remapping requires local handwritten Rust files: " + source)
        canonical = package + "/" + source
        staged = _BIN_ROOT + "/" + canonical
        for prefix in ["", _ABSOLUTE_ROOT]:
            flags.append("--remap-path-prefix=" + prefix + staged + "=" + canonical)
    return flags
