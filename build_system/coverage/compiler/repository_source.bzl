"""Pin, patch and native tool checks; no compilation or profile mutation."""

load(":repository_common.bzl", "digests", "execute", "retained_file")

_PATCH_PATHS = {
    "0001-direct-counter-mode.patch": [
        "compiler/rustc_mir_transform/src/coverage/graph.rs",
        "compiler/rustc_mir_transform/src/coverage/mod.rs",
        "compiler/rustc_mir_transform/src/coverage/tests.rs",
        "compiler/rustc_session/src/options.rs",
    ],
    "0002-physical-bcb-counters.patch": [
        "compiler/rustc_mir_transform/src/coverage/counters.rs",
        "compiler/rustc_mir_transform/src/coverage/query.rs",
    ],
}

def verify_source(ctx, locations, pin):
    """Require the exact source commit and byte-identical tracked patches.

    Args:
      ctx: Repository context.
      locations: Canonical source and workspace paths.
      pin: Parsed compiler pin.
    Returns:
      Patch name to SHA-256 mapping.
    """
    source = locations.source
    git = ["/usr/bin/git", "-C", str(source)]
    retained_file(ctx, source, ".git/HEAD")
    if execute(ctx, git + ["rev-parse", "HEAD"]) != pin["commit"]:
        fail("coverage compiler source HEAD differs from pin.toml")
    patches = {ctx.path(label).basename: ctx.path(label) for label in ctx.attr.patches}
    if sorted(patches) != sorted(pin["patches"]) or sorted(patches) != sorted(_PATCH_PATHS):
        fail("coverage patch inventory differs from pinned patch order")
    expected_paths = []
    for name in pin["patches"]:
        paths = _PATCH_PATHS[name]
        expected_paths.extend(paths)
        for path in paths:
            retained_file(ctx, source, path)
        actual = execute(ctx, git + ["diff", "HEAD", "--no-ext-diff", "--binary", "--"] + paths)
        if actual != ctx.read(patches[name]).strip():
            fail("prepared compiler differs from recorded patch: " + name)
    modified = execute(ctx, git + ["diff", "HEAD", "--name-only", "--ignore-submodules"]).splitlines()
    if sorted(modified) != sorted(expected_paths):
        fail("compiler source has unrecorded tracked changes")
    for path, commit in pin["submodule_commits"].items():
        if execute(ctx, git + ["-C", path, "rev-parse", "HEAD"]) != commit:
            fail("coverage compiler submodule differs from pin: " + path)
    return digests(ctx, patches)

def verify_tools(ctx, source, pin):
    """Inspect native tool identities without compiling a source file.

    Args:
      ctx: Repository context.
      source: Canonical source root.
      pin: Parsed compiler pin.
    Returns:
      Actual rustc, rustdoc and LLVM version output.
    """
    base = "build/" + pin["host"]
    rustc = retained_file(ctx, source, base + "/stage1/bin/rustc")
    rustdoc = retained_file(ctx, source, base + "/stage1/bin/rustdoc")
    verbose = execute(ctx, [str(rustc), "-vV"])
    expected = {
        "host": pin["host"],
        "release": pin["version"] + "-dev",
        "LLVM version": pin["llvm_version"],
    }
    fields = {}
    for line in verbose.splitlines()[1:]:
        parts = line.split(": ", 1)
        if len(parts) == 2:
            fields[parts[0]] = parts[1]
    for key, value in expected.items():
        if fields.get(key) != value:
            fail("unexpected coverage rustc " + key)
    if verbose.splitlines()[0] != "rustc {}-dev ({})".format(pin["version"], pin["description"]):
        fail("coverage compiler brand differs from pin")
    rustdoc_version = execute(ctx, [str(rustdoc), "--version"])
    if rustdoc_version != "rustdoc {}-dev ({})".format(pin["version"], pin["description"]):
        fail("rustdoc is not from the matching branded build")
    if ctx.path(execute(ctx, [str(rustc), "--print", "sysroot"])).realpath != source.get_child(base + "/stage1").realpath:
        fail("compiler resolved a foreign sysroot")
    if "coverage-direct-counters" not in execute(ctx, [str(rustc), "-Z", "help"]):
        fail("compiler does not implement the direct-counter flag")
    llvm_versions = {}
    for tool in ["llvm-cov", "llvm-profdata"]:
        binary = retained_file(ctx, source, base + "/llvm/bin/" + tool)
        llvm_versions[tool] = execute(ctx, [str(binary), "--version"])
        versions = [line.strip() for line in llvm_versions[tool].splitlines()]
        expected = "LLVM version " + pin["llvm_version"]
        if expected not in versions and expected + "-rust-dev" not in versions:
            fail("coverage LLVM tool has a foreign version: " + tool)

    # Preserve rustc's real commit-hash (including unknown); do not invent one.
    return struct(rustc = verbose, rustdoc = rustdoc_version, llvm = llvm_versions)
