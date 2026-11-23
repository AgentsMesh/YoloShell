"""Repository source inventory that never descends into generated output roots."""

_IGNORED_ROOTS = [
    ".build",
    ".git",
    ".verification",
    ".yolo-dev",
    "bazel-*",
    "coverage",
    "dist",
    "target",
    "test-results",
]

_SOURCE_PATTERNS = [
    "*.bash",
    "*.bazel",
    "*.bzl",
    "*.capnp",
    "*.cjs",
    "*.css",
    "*.exp",
    "*.go",
    "*.h",
    "*.html",
    "*.js",
    "*.json",
    "*.jsonl",
    "*.jsx",
    "*.md",
    "*.mjs",
    "*.modulemap",
    "*.patch",
    "*.plist",
    "*.rs",
    "*.sh",
    "*.toml",
    "*.ts",
    "*.tsx",
    "*.yaml",
    "*.yml",
    "*.zsh",
    "CMakeLists.txt",
    "build.gradle",
    "build.gradle.kts",
    "go.mod",
    "go.sum",
    "go.work",
    "meson.build",
]

_ROOT_CONFIGURATION = [
    ".bazelignore",
    ".bazelrc",
    ".bazelversion",
    ".gitattributes",
    ".gitignore",
    "Cargo.lock",
]

def root_architecture_sources():
    files = native.glob(["*"], exclude = _IGNORED_ROOTS)
    roots = [
        entry
        for entry in native.glob(["*"], exclude = _IGNORED_ROOTS, exclude_directories = 0)
        if entry not in files
    ]
    patterns = _SOURCE_PATTERNS + [
        root + "/**/" + pattern
        for root in roots
        for pattern in _SOURCE_PATTERNS
    ]
    extensionless = native.glob(
        ["*"] + [root + "/**/*" for root in roots],
        allow_empty = True,
        exclude = _IGNORED_ROOTS + ["**/*.*"] + _ROOT_CONFIGURATION,
    )
    return native.glob(patterns, allow_empty = True) + extensionless + _ROOT_CONFIGURATION
