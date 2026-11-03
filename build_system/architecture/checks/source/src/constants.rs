pub const MAX_HANDWRITTEN_LINES: usize = 199;
pub const BUILD_SYSTEM_PREFIX: &str = "build_system/";
pub const UNIT_TEST_DIRECTORY_NAME: &str = "unit_tests";
pub const INTEGRATION_TEST_DIRECTORY_NAME: &str = "integration_tests";
pub const TEST_SUPPORT_DIRECTORY_NAME: &str = "test_support";
pub const RUST_TEST_FILE_SUFFIXES: &[&str] = &["_test", "_tests"];
pub const TEST_SUPPORT_FILE_STEMS: &[&str] = &["constants", "fixture", "support"];

pub const HANDWRITTEN_EXTENSIONS: &[&str] = &[
    "bash",
    "bazel",
    "bzl",
    "capnp",
    "css",
    "exp",
    "h",
    "html",
    "json",
    "jsonl",
    "md",
    "modulemap",
    "patch",
    "plist",
    "rs",
    "sh",
    "swift",
    "toml",
    "ts",
    "tsx",
    "yaml",
    "yml",
    "zsh",
];
pub const HANDWRITTEN_FILE_NAMES: &[&str] = &[
    ".bazelignore",
    ".bazelrc",
    ".bazelversion",
    ".gitattributes",
    ".gitignore",
    "BUILD",
];
pub const GENERATED_TEXT_FILE_NAMES: &[&str] =
    &["Cargo.lock", "MODULE.bazel.lock", "Package.resolved"];
pub const RETIRED_SCRIPT_EXTENSIONS: &[&str] = &["cjs", "js", "jsx", "mjs", "ts", "tsx"];
pub const RETIRED_FRONTEND_FILE_PARTS: &[&[&str]] = &[
    &["package", ".json"],
    &["package", "-lock.json"],
    &["pn", "pm-lock.yaml"],
    &["pn", "pm-workspace.yaml"],
    &["yarn", ".lock"],
    &["bun", ".lock"],
    &["bun", ".lockb"],
    &["deno", ".json"],
    &["deno", ".jsonc"],
    &["npm", "-shrinkwrap.json"],
    &["ts", "config.json"],
    &["js", "config.json"],
];
pub const RETIRED_FRONTEND_DIRECTORY_PARTS: &[&[&str]] =
    &[&["no", "de", "_", "modules"], &["play", "wright-report"]];

#[derive(Clone, Copy)]
pub enum MatchStyle {
    Identifier,
    Package,
    Runtime,
    Substring,
}

pub struct FragmentedTerm {
    pub parts: &'static [&'static str],
    pub style: MatchStyle,
}

pub const RETIRED_NAME_PARTS: &[&str] = &["con", "flux"];
pub const FRONTEND_TERMS: &[FragmentedTerm] = &[
    FragmentedTerm {
        parts: &["elec", "tron"],
        style: MatchStyle::Substring,
    },
    FragmentedTerm {
        parts: &["no", "de"],
        style: MatchStyle::Runtime,
    },
    FragmentedTerm {
        parts: &["re", "act"],
        style: MatchStyle::Identifier,
    },
    FragmentedTerm {
        parts: &["x", "term"],
        style: MatchStyle::Package,
    },
    FragmentedTerm {
        parts: &["play", "wright"],
        style: MatchStyle::Substring,
    },
    FragmentedTerm {
        parts: &["vi", "te"],
        style: MatchStyle::Identifier,
    },
    FragmentedTerm {
        parts: &["vi", "test"],
        style: MatchStyle::Substring,
    },
    FragmentedTerm {
        parts: &["pn", "pm"],
        style: MatchStyle::Substring,
    },
    FragmentedTerm {
        parts: &["web", "view"],
        style: MatchStyle::Substring,
    },
    FragmentedTerm {
        parts: &["swift", "ui"],
        style: MatchStyle::Substring,
    },
];
