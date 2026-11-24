use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const RESOLUTION_POLICY: &str = "retained authority resolution is indeterminate";
const MODULE_NAME: &str = "main";
const RETAINED_SOURCE: &str = r#"
struct RetainedExactProcess;
struct DurableContainment { target: RetainedExactProcess }
fn bypass(target: &mut DurableContainment) { target.enforce_containment(); }
"#;

#[test]
fn main_module_apparent_label_resolves_to_the_local_crate_root() {
    let fixture = fixture_with_module(&format!("module(name = \"{MODULE_NAME}\")\n"));
    fixture.write(
        "tools/runtime/BUILD.bazel",
        format!(
            "rust_library(name = \"guarded\", crate_root = \"@{MODULE_NAME}//tools/runtime:src/lib.rs\")\n"
        ),
    );
    fixture.write("tools/runtime/src/lib.rs", RETAINED_SOURCE);
    assert_has(&messages(&fixture), CONSTRUCTOR_POLICY);
}

#[test]
fn explicit_main_repository_name_resolves_to_the_local_crate_root() {
    let fixture = fixture_with_module(&format!(
        "module(name = \"module_name\", repo_name = \"{MODULE_NAME}\")\n"
    ));
    fixture.write(
        "tools/runtime/BUILD.bazel",
        format!(
            "rust_library(name = \"guarded\", crate_root = \"@{MODULE_NAME}//tools/runtime:src/lib.rs\")\n"
        ),
    );
    fixture.write("tools/runtime/src/lib.rs", RETAINED_SOURCE);
    assert_has(&messages(&fixture), CONSTRUCTOR_POLICY);
}

#[test]
fn uncertain_main_module_evidence_fails_closed_for_apparent_labels() {
    for definition in [
        "module(version = \"1.0\")\n",
        "module(name = selected_name())\n",
        "module(name = \"module_name\", repo_name = selected_name())\n",
        "module(name = \"module_name\", repo_name = \"one\", repo_name = \"main\")\n",
        "module(name = \"one\")\nmodule(name = \"two\")\n",
    ] {
        let fixture = fixture_with_module(definition);
        fixture.write(
            "core/BUILD.bazel",
            format!(
                "rust_library(name = \"guarded\", crate_root = \"@{MODULE_NAME}//core:src/lib.rs\")\n"
            ),
        );
        fixture.write("core/src/lib.rs", RETAINED_SOURCE);
        assert_has(&messages(&fixture), RESOLUTION_POLICY);
    }
}

#[test]
fn missing_module_evidence_fails_closed_for_apparent_labels() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/BUILD.bazel",
        format!(
            "rust_library(name = \"guarded\", crate_root = \"@{MODULE_NAME}//core:src/lib.rs\")\n"
        ),
    );
    fixture.write("core/src/lib.rs", RETAINED_SOURCE);
    assert_has(&messages(&fixture), RESOLUTION_POLICY);
}

#[test]
fn proven_external_repository_roots_do_not_pollute_local_resolution() {
    let fixture = fixture_with_module(&format!("module(name = \"{MODULE_NAME}\")\n"));
    fixture.write(
        "core/BUILD.bazel",
        "rust_library(name = \"external\", crate_root = \"@dependency//pkg:lib.rs\")\n",
    );
    let result = messages(&fixture);
    assert_lacks(&result, CONSTRUCTOR_POLICY);
    assert_lacks(&result, RESOLUTION_POLICY);
}

fn fixture_with_module(definition: &str) -> Fixture {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write("MODULE.bazel", definition);
    fixture
}

fn assert_has(result: &[String], expected: &str) {
    assert!(
        result.iter().any(|message| message.contains(expected)),
        "{result:#?}"
    );
}

fn assert_lacks(result: &[String], expected: &str) {
    assert!(
        result.iter().all(|message| !message.contains(expected)),
        "{result:#?}"
    );
}
