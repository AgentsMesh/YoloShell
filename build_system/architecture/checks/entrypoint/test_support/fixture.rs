use super::support::{inspect, messages, source};
use crate::constants as contract;
use yolo_architecture_foundation::SourceFile;

pub const ENTRY: &str = r#"
rust_binary(
    name = "yolo", crate_root = "src/main.rs", srcs = glob(["src/*.rs"]),
    deps = ["//core/crates/application/launch/contract:yolo_launch_contract", "@crates//:anyhow", "@crates//:clap"],
)
"#;
pub const MAIN: &str = "core/crates/application/yolo/src/main.rs";
pub const MAIN_SOURCE: &str = "fn main() {}\n";
pub const TEST_BINARY: &str = r#"
rust_binary(name = "fixture", testonly = True, crate_root = "src/main.rs", srcs = ["src/main.rs"])
"#;
pub const OTHER_BUILD: &str = "core/crates/other/BUILD.bazel";
pub const OTHER_MAIN: &str = "core/crates/other/src/main.rs";
pub const BUSINESS: &str = "//core/crates/runtime/engine/host:yolo_host";
pub const ALLOWED: &str = "@crates//:anyhow";
pub const FORBIDDEN_RUNTIME: &str = "@crates//:tokio";
use super::constants::{CAPNP_DEFINITIONS, CAPNP_GENERATION};

pub fn files(build: impl Into<String>) -> Vec<SourceFile> {
    vec![
        source(yolo_architecture_target_ownership::OWNERSHIP_POLICY_BUILD_FILE, ""),
        source(contract::BUILD, build),
        source(MAIN, MAIN_SOURCE),
    ]
}

pub fn report(build: impl Into<String>) -> String {
    messages(&inspect(files(build)))
}

pub fn schema_files(build: impl Into<String>) -> Vec<SourceFile> {
    let mut inventory = files(ENTRY);
    inventory.extend([
        source(OTHER_BUILD, build),
        source(contract::CAPNP_DEFINITIONS, CAPNP_DEFINITIONS),
        source(contract::CAPNP_GENERATION, CAPNP_GENERATION),
    ]);
    inventory
}

pub fn binary(name: &str, attribute: &str) -> String {
    format!(
        "rust_binary(name = {name:?}, {attribute} crate_root = \"src/main.rs\", srcs = [\"src/main.rs\"])\n"
    )
}
