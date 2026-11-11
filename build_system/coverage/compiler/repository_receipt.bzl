"""Content-addressed import receipt for a complete local stage1 build."""

load(":repository_artifacts.bzl", "inventory")
load(":repository_common.bzl", "RECEIPT_SCHEMA", "digests", "read_pin", "roots")
load(":repository_source.bzl", "verify_source", "verify_tools")

def observe_build(ctx):
    """Capture source and actual artifact digests as a versioned receipt.

    Args:
      ctx: Repository context with pinned inputs.
    Returns:
      Complete receipt and the artifact paths it describes.
    """
    pin = read_pin(ctx)
    locations = roots(ctx, pin)
    patch_digests = verify_source(ctx, locations, pin)
    tools = verify_tools(ctx, locations.source, pin)
    artifacts = inventory(ctx, locations.source, pin, tools.rustc)
    metadata = digests(ctx, {
        "bootstrap": ctx.path(ctx.attr.bootstrap),
        "pin": ctx.path(ctx.attr.pin),
        "stdlib_build_record": artifacts.build_record,
    })
    receipt = {
        "schema_version": RECEIPT_SCHEMA,
        "source_root": locations.relative,
        "source_commit": pin["commit"],
        "submodule_commits": pin["submodule_commits"],
        "brand": pin["description"],
        "host": pin["host"],
        "llvm_version": pin["llvm_version"],
        "rustc_verbose_version": tools.rustc,
        "rustdoc_version": tools.rustdoc,
        "llvm_tool_versions": tools.llvm,
        "patch_sha256": patch_digests,
        "metadata_sha256": metadata,
        "artifact_sha256": digests(ctx, artifacts.files),
    }
    return struct(receipt = receipt, files = artifacts.files)
