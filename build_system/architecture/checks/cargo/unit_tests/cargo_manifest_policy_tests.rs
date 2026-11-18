use super::cargo_manifest_test_support::metadata_manifest;
use crate::policy_constants::{
    CARGO_FALSE_LITERAL, CARGO_METADATA_SHAPE_MESSAGE, CARGO_PRODUCTION_TARGET_MESSAGE,
    CARGO_REQUIRED_LIBRARY_FALSE_KEYS, CARGO_REQUIRED_LIBRARY_STRINGS,
    CARGO_REQUIRED_PACKAGE_FALSE_KEYS,
};
use yolo_architecture_test_support::Fixture;

const REPLACEMENT_STRING_VALUE: &str = "replacement";

fn messages(source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", source);
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn accepts_the_fixed_dependency_metadata_target() {
    assert!(messages(&metadata_manifest("")).is_empty());
}

#[test]
fn accepts_comments_and_hash_characters_inside_strings() {
    let source = replace_line(
        &metadata_manifest("description = \"hash # remains data\"\n"),
        "autobins = false",
        "\"autobins\" = false # implicit binaries stay disabled",
    );
    assert!(messages(&source).is_empty());
}

#[test]
fn rejects_explicit_targets_profiles_and_workspaces() {
    let source = format!(
        "{}\n[[bin]]\nname = \"app\"\n[[test]]\nname = \"suite\"\n[[example]]\nname = \"sample\"\n[[bench]]\nname = \"speed\"\n[profile.release]\nlto = true\n[workspace]\nmembers = []\n",
        metadata_manifest("")
    );
    let result = messages(&source);
    assert_eq!(result.len(), 6, "{result:?}");
    assert!(
        result
            .iter()
            .all(|entry| entry.contains(CARGO_PRODUCTION_TARGET_MESSAGE))
    );
}

#[test]
fn rejects_quoted_and_spaced_target_table_bypasses() {
    let source = format!(
        "{}\n[[ \"bin\" ]]\nname = \"app\"\n[ \"profile\" . \"release\" ]\nlto = true\n[ \"workspace\" . \"dependencies\" ]\n",
        metadata_manifest("")
    );
    let result = messages(&source);
    assert_eq!(result.len(), 3, "{result:?}");
    assert!(
        result
            .iter()
            .all(|entry| entry.contains(CARGO_PRODUCTION_TARGET_MESSAGE))
    );
}

#[test]
fn rejects_every_required_false_setting_independently() {
    for key in CARGO_REQUIRED_PACKAGE_FALSE_KEYS
        .iter()
        .chain(CARGO_REQUIRED_LIBRARY_FALSE_KEYS)
    {
        let from = format!("{key} = {CARGO_FALSE_LITERAL}");
        let to = format!("{key} = true");
        assert_single_shape(&replace_line(&metadata_manifest(""), &from, &to));
    }
}

#[test]
fn rejects_quoted_false_build_scripts_and_changed_stub_identity() {
    let quoted_false = replace_line(
        &metadata_manifest(""),
        "autobins = false",
        "autobins = \"false\"",
    );
    assert_single_shape(&quoted_false);
    let build_script = replace_line(
        &metadata_manifest(""),
        "build = false",
        "build = \"build.rs\"",
    );
    assert_single_shape(&build_script);
    for (key, expected) in CARGO_REQUIRED_LIBRARY_STRINGS {
        let from = format!("{key} = \"{expected}\"");
        let to = format!("{key} = \"{REPLACEMENT_STRING_VALUE}\"");
        assert_single_shape(&replace_line(&metadata_manifest(""), &from, &to));
    }
}

#[test]
fn rejects_duplicate_keys_tables_array_tables_and_multiline_values() {
    let duplicate_key = replace_line(
        &metadata_manifest(""),
        "autobins = false",
        "autobins = false\nautobins = false",
    );
    assert_single_shape(&duplicate_key);
    assert_single_shape(&format!("{}\n[lib]\n", metadata_manifest("")));
    assert_single_shape(&format!("{}\n[[lib]]\n", metadata_manifest("")));
    let multiline = metadata_manifest("description = \"\"\"metadata\"\"\"\n");
    assert_single_shape(&multiline);
}

#[test]
fn rejects_dotted_inline_and_escaped_root_bypasses() {
    let source = format!(
        "profile.release.lto = true\nworkspace = {{ members = [] }}\nbin = [{{ name = \"app\" }}]\n{}",
        metadata_manifest("")
    );
    let result = messages(&source);
    assert_eq!(result.len(), 3, "{result:?}");
    assert!(
        result
            .iter()
            .all(|entry| entry.contains(CARGO_PRODUCTION_TARGET_MESSAGE))
    );
    let escaped = format!(
        "[\"pro\\u0066ile\".release]\nlto = true\n{}",
        metadata_manifest("")
    );
    assert_single_shape(&escaped);
}

fn assert_single_shape(source: &str) {
    let result = messages(source);
    assert_eq!(result.len(), 1, "{result:?}");
    assert!(result[0].contains(CARGO_METADATA_SHAPE_MESSAGE));
}

fn replace_line(source: &str, from: &str, to: &str) -> String {
    source
        .lines()
        .map(|line| if line == from { to } else { line })
        .collect::<Vec<_>>()
        .join("\n")
}
