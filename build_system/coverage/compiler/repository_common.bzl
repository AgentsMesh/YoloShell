"""Read-only validation shared by the local build receipt and toolchain import."""

ROOT_ENV = "YOLO_COVERAGE_RUSTC_ROOT"
RECEIPT_SCHEMA = 1
RECEIPT_FILE = "build-receipt.json"

def execute(ctx, arguments):
    result = ctx.execute(arguments, quiet = True, timeout = 60)
    if result.return_code:
        fail("coverage compiler validation failed: {}\n{}".format(arguments, result.stderr))
    return result.stdout.strip()

def read_pin(ctx):
    """Read the repository-owned, deliberately restricted TOML pin document.

    Args:
      ctx: Repository context with a pin label.
    Returns:
      Parsed scalar values and named string maps.
    """

    # pin.toml deliberately uses JSON-compatible scalars/lists and string maps.
    result = {}
    selected = result
    for raw in ctx.read(ctx.attr.pin).splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            name = line[1:-1]
            if name in result:
                fail("duplicate compiler pin section: " + name)
            result[name] = {}
            selected = result[name]
            continue
        parts = line.split("=", 1)
        if len(parts) != 2:
            fail("unsupported compiler pin syntax")
        key = parts[0].strip()
        key = json.decode(key) if key.startswith('"') else key
        if key in selected:
            fail("duplicate compiler pin field: " + key)
        selected[key] = json.decode(parts[1].strip())
    return result

def roots(ctx, pin):
    """Resolve the explicit source directory inside the main checkout.

    Args:
      ctx: Repository context and explicit repository environment.
      pin: Parsed compiler pin.
    Returns:
      Canonical workspace, source and workspace-relative source paths.
    """
    workspace = ctx.path(ctx.attr.pin).dirname.dirname.dirname.dirname.realpath
    configured = ctx.getenv(ROOT_ENV)
    if not configured:
        fail("set --repo_env={}={} to import the prepared compiler".format(ROOT_ENV, pin["source_directory"]))
    source = ctx.path(configured if configured.startswith("/") else str(workspace.get_child(configured))).realpath
    if not str(source).startswith(str(workspace.get_child(".build")) + "/"):
        fail("coverage compiler must be inside this checkout's canonical .build directory")
    return struct(workspace = workspace, source = source, relative = str(source)[len(str(workspace)) + 1:])

def retained_file(ctx, root, relative):
    """Watch a required file and reject a link into a foreign toolchain.

    Args:
      ctx: Repository context.
      root: Canonical source root.
      relative: Required file path relative to root.
    Returns:
      The watched file path.
    """
    path = root.get_child(relative)
    if not path.exists or path.is_dir:
        fail("missing coverage compiler artifact: {}\nBuild library and src/tools/rustdoc together.".format(path))
    if not str(path.realpath).startswith(str(root.realpath) + "/"):
        fail("coverage artifact escapes its pinned source tree: " + str(path))
    ctx.watch(path)
    return path

def digests(ctx, files):
    """Return SHA-256 by logical artifact name, checking every reported path.

    Args:
      ctx: Repository context.
      files: Logical name to file path mapping.
    Returns:
      Logical name to hexadecimal digest mapping.
    """
    if not files:
        return {}
    names = sorted(files)
    for name in names:
        ctx.watch(files[name])
    output = execute(ctx, ["/usr/bin/shasum", "-a", "256", "--"] + [str(files[name]) for name in names])
    lines = output.splitlines()
    if len(lines) != len(names):
        fail("incomplete artifact digest output")
    result = {}
    for name, line in zip(names, lines):
        digest = line[:64]
        if len(digest) != 64 or not all([char in "0123456789abcdef" for char in digest.elems()]):
            fail("invalid artifact digest")
        if line[64:] != "  " + str(files[name]):
            fail("artifact digest path mismatch")
        result[name] = digest
    return result

def directory_files(ctx, source, relative, suffixes):
    """Watch a flat artifact directory and select declared file suffixes.

    Args:
      ctx: Repository context.
      source: Canonical source root.
      relative: Artifact directory relative to source.
      suffixes: Allowed artifact filename suffixes.
    Returns:
      Source-relative names mapped to watched files.
    """
    directory = source.get_child(relative)
    if not directory.exists or not directory.is_dir:
        fail("missing compiler directory: " + str(directory))
    # The sysroot contains rustlib/src -> the source tree. Watch only this flat
    # directory and its selected files; recursive watching would follow a cycle.
    ctx.watch(directory)
    return {
        relative + "/" + path.basename: retained_file(ctx, source, relative + "/" + path.basename)
        for path in directory.readdir(watch = "yes")
        if not path.is_dir and any([path.basename.endswith(suffix) for suffix in suffixes])
    }
