"""Inventory only the prepared compiler, its own std build, and native LLVM tools."""

load(":repository_common.bzl", "digests", "directory_files", "retained_file")

def inventory(ctx, source, pin, verbose):
    """Bind the complete native std inventory to its stage1 build output.

    Args:
      ctx: Repository context.
      source: Canonical source root.
      pin: Parsed compiler pin.
      verbose: Verified rustc verbose version output.
    Returns:
      Imported artifact paths and matching std build-record path.
    """
    base = "build/" + pin["host"]
    stage = base + "/stage1/"
    libraries = "lib/rustlib/" + pin["host"] + "/lib"
    files = {
        "bin/rustc": retained_file(ctx, source, stage + "bin/rustc"),
        "bin/rustdoc": retained_file(ctx, source, stage + "bin/rustdoc"),
    }
    for relative, file in directory_files(ctx, source, stage + "lib", [".dylib"]).items():
        files[relative[len(stage):]] = file
    std = directory_files(ctx, source, stage + libraries, [".rlib", ".rmeta", ".dylib", ".a"])
    contained = source.get_child(stage + libraries + "/self-contained")
    if contained.exists:
        ctx.watch_tree(contained)
        if contained.readdir():
            fail("this native macOS import does not support a nonempty self-contained std directory")
    for stem in ["libstd-", "libcore-", "libtest-", "libprofiler_builtins-"]:
        if not any([path.basename.startswith(stem) and path.basename.endswith(".rlib") for path in std.values()]):
            fail("matching stage1 std is incomplete: missing " + stem)

    # Match every staged std artifact against the actual stage1 std build output.
    built = {}
    for relative, file in std.items():
        logical = relative[len(stage):]
        files[logical] = file
        built[logical] = retained_file(ctx, source, base + "/stage1-std/" + pin["host"] + "/release/deps/" + file.basename)
    if digests(ctx, built) != digests(ctx, {relative[len(stage):]: file for relative, file in std.items()}):
        fail("staged std differs from the matching stage1 std build")
    info_path = retained_file(ctx, source, base + "/stage1-std/.rustc_info.json")
    build_info = json.decode(ctx.read(info_path))
    if not any([entry.get("success") and entry.get("stdout", "").strip() == verbose for entry in build_info.get("outputs", {}).values()]):
        fail("std build record does not identify this branded rustc")
    for tool in ["llvm-cov", "llvm-profdata"]:
        files["llvm/bin/" + tool] = retained_file(ctx, source, base + "/llvm/bin/" + tool)
    for relative, file in directory_files(ctx, source, base + "/llvm/lib", [".dylib"]).items():
        files["llvm/lib/" + file.basename] = file
    return struct(files = files, build_record = info_path)
