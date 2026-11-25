use yolo_architecture_repository::repository_rules;
use yolo_architecture_test_support::Fixture;

#[path = "module_scope_tests.rs"]
mod module_scope_tests;

const BASELINE: &str = "build_system/architecture/include_composition_baseline_runtime.bzl";
const ASSIGNMENT: &str = "INCLUDE_COMPOSITION_BASELINE";

fn messages(fixture: &Fixture) -> Vec<String> {
    let files = yolo_architecture_test_support::load(fixture.root()).unwrap();
    let mut violations = Vec::new();
    repository_rules::inspect(&files, &[], &mut violations);
    violations
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

fn composition_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("core/src/part.rs", "pub fn placeholder() {}\n");
    fixture.write("core/src/lib.rs", "include!(\"part.rs\");\n");
    fixture
}

fn baseline(entries: &str) -> String {
    format!("{ASSIGNMENT} = [\n{entries}]\n")
}

fn single(messages: &[String]) -> String {
    assert_eq!(messages.len(), 1, "{messages:?}");
    messages[0].clone()
}

#[test]
fn rejects_unrecorded_composition_site() {
    let fixture = composition_fixture();

    let result = single(&messages(&fixture));
    assert!(result.contains("include! composition"), "{result:?}");
    assert!(result.contains("core/src/lib.rs"), "{result:?}");
}

#[test]
fn admits_recorded_composition_site() {
    let fixture = composition_fixture();
    fixture.write(
        BASELINE,
        &baseline("    (\"core/src/lib.rs\", \"part.rs\"),\n"),
    );

    assert!(messages(&fixture).is_empty());
}

#[test]
fn admits_recorded_site_with_a_different_relative_spelling() {
    let fixture = composition_fixture();
    fixture.write("core/src/lib.rs", "include!(\"./part.rs\");\n");
    fixture.write(
        BASELINE,
        &baseline("    (\"core/src/lib.rs\", \"part.rs\"),\n"),
    );

    assert!(messages(&fixture).is_empty());
}

#[test]
fn admits_generated_and_test_targets_without_recording() {
    let fixture = Fixture::new();
    fixture.write(
        "core/src/generated/contract.rs",
        "pub fn placeholder() {}\n",
    );
    fixture.write("core/src/lib.rs", "include!(\"generated/contract.rs\");\n");
    fixture.write("core/unit_tests/wiring.rs", "mod part;\n");
    fixture.write("core/unit_tests/part.rs", "pub fn placeholder() {}\n");
    fixture.write(
        "core/src/tests.rs",
        "include!(\"../unit_tests/wiring.rs\");\n",
    );

    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_path_traversal_through_a_test_directory() {
    let fixture = composition_fixture();
    fixture.write(
        "core/src/lib.rs",
        "include!(\"../unit_tests/../evil.rs\");\n",
    );

    let result = single(&messages(&fixture));
    assert!(result.contains("include! composition"), "{result:?}");
}

#[test]
fn rejects_a_test_target_from_another_package() {
    let fixture = composition_fixture();
    fixture.write(
        "core/src/lib.rs",
        "include!(\"../../other/unit_tests/wiring.rs\");\n",
    );

    let result = single(&messages(&fixture));
    assert!(result.contains("include! composition"), "{result:?}");
}

#[test]
fn rejects_a_dynamic_composition_argument() {
    let fixture = composition_fixture();
    fixture.write(
        "core/src/lib.rs",
        "include!(concat!(env!(\"OUT_DIR\"), \"/part.rs\"));\n",
    );

    let result = single(&messages(&fixture));
    assert!(result.contains("literal path"), "{result:?}");
}

#[test]
fn rejects_an_unrecorded_brace_delimited_site() {
    let fixture = composition_fixture();
    fixture.write("core/src/lib.rs", "include! { \"part.rs\" }\n");

    let result = single(&messages(&fixture));
    assert!(result.contains("include! composition"), "{result:?}");
}

#[test]
fn rejects_stale_composition_baseline_entry() {
    let fixture = Fixture::new();
    fixture.write(
        BASELINE,
        &baseline("    (\"core/src/lib.rs\", \"removed.rs\"),\n"),
    );

    let result = single(&messages(&fixture));
    assert!(result.contains("stale"), "{result:?}");
    assert!(result.contains("removed.rs"), "{result:?}");
}

#[test]
fn rejects_malformed_composition_baseline_entry() {
    let fixture = Fixture::new();
    fixture.write(BASELINE, &baseline("    (\"core/src/lib.rs\"),\n"));

    let result = single(&messages(&fixture));
    assert!(result.contains("source and target"), "{result:?}");
}

#[test]
fn rejects_a_test_file_that_includes_production_code() {
    let fixture = Fixture::new();
    fixture.write("core/src/lib.rs", "include!(\"../unit_tests/wiring.rs\");\n");
    fixture.write("core/unit_tests/wiring.rs", "include!(\"../src/part.rs\");\n");
    fixture.write("core/src/part.rs", "pub fn placeholder() {}\n");

    let result = single(&messages(&fixture));
    assert!(result.contains("include! composition"), "{result:?}");
    assert!(result.contains("core/unit_tests/wiring.rs"), "{result:?}");
}

#[test]
fn admits_test_wiring_within_its_package() {
    let fixture = Fixture::new();
    fixture.write("core/src/lib.rs", "include!(\"../unit_tests/wiring.rs\");\n");
    fixture.write("core/unit_tests/wiring.rs", "include!(\"part.rs\");\n");
    fixture.write("core/unit_tests/part.rs", "pub fn placeholder() {}\n");

    assert!(messages(&fixture).is_empty());
}
