use yolo_architecture_policy_contracts::constants::*;

const FIXTURE_PACKAGE_NAME: &str = "fixture";
const FIXTURE_PACKAGE_VERSION: &str = "0.0.0";

pub fn metadata_manifest(package_declarations: &str) -> String {
    format!(
        "[{CARGO_PACKAGE_TABLE}]\nname = \"{FIXTURE_PACKAGE_NAME}\"\nversion = \"{FIXTURE_PACKAGE_VERSION}\"\n{package_declarations}{}\n[{CARGO_LIBRARY_TABLE}]\n{}\n{}",
        false_entries(CARGO_REQUIRED_PACKAGE_FALSE_KEYS),
        string_entries(CARGO_REQUIRED_LIBRARY_STRINGS),
        false_entries(CARGO_REQUIRED_LIBRARY_FALSE_KEYS),
    )
}

fn string_entries(requirements: &[(&str, &str)]) -> String {
    requirements
        .iter()
        .map(|(key, value)| format!("{key} = \"{value}\""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn false_entries(keys: &[&str]) -> String {
    keys.iter()
        .map(|key| format!("{key} = {CARGO_FALSE_LITERAL}"))
        .collect::<Vec<_>>()
        .join("\n")
}
