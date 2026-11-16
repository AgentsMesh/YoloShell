use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const RESOLUTION_POLICY: &str = "retained authority resolution is indeterminate";
const RETAINED_SOURCE: &str = r#"
struct RetainedExactProcess;
pub struct DurableContainment { target: RetainedExactProcess }
"#;
const RETAINED_CONSUMER: &str = r#"
fn bypass(target: &mut crate::authority::DurableContainment) {
    target.enforce_containment();
}
"#;
const CLEAN_SOURCE: &str = "pub fn clean() {}\n";

#[test]
fn included_fragment_module_uses_the_fragment_directory() {
    let fixture = module_fixture("process/platform_module_wiring.rs");
    fixture.write(
        "core/src/process/platform_module_wiring.rs",
        "mod process_events_macos;\n",
    );
    fixture.write(
        "core/src/process/process_events_macos.rs",
        RETAINED_CONSUMER,
    );
    fixture.write(
        "core/src/process/platform_module_wiring/process_events_macos.rs",
        CLEAN_SOURCE,
    );
    assert_detected_without_uncertainty(&messages(&fixture));
}

#[test]
fn included_fragment_module_ignores_the_fragment_stem_directory() {
    let fixture = module_fixture("process/platform_module_wiring.rs");
    fixture.write(
        "core/src/process/platform_module_wiring.rs",
        "mod process_events_macos;\n",
    );
    fixture.write("core/src/process/process_events_macos.rs", CLEAN_SOURCE);
    fixture.write(
        "core/src/process/platform_module_wiring/process_events_macos.rs",
        RETAINED_CONSUMER,
    );
    let result = messages(&fixture);
    assert_lacks(&result, CONSTRUCTOR_POLICY);
    assert_lacks(&result, RESOLUTION_POLICY);
}

#[test]
fn nested_include_module_uses_the_innermost_fragment_directory() {
    let fixture = module_fixture("parts/a.rs");
    fixture.write("core/src/parts/a.rs", "include!(\"nested/b.rs\");\n");
    fixture.write("core/src/parts/nested/b.rs", "mod selected;\n");
    fixture.write("core/src/parts/nested/selected.rs", RETAINED_CONSUMER);
    fixture.write("core/src/parts/nested/b/selected.rs", CLEAN_SOURCE);
    assert_detected_without_uncertainty(&messages(&fixture));
}

fn module_fixture(include: &str) -> Fixture {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/BUILD.bazel",
        "rust_library(name = \"root\", crate_root = \"src/lib.rs\")\n",
    );
    fixture.write(
        "core/src/lib.rs",
        format!("mod authority; include!(\"{include}\");\n"),
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture
}

fn assert_detected_without_uncertainty(result: &[String]) {
    assert_has(result, CONSTRUCTOR_POLICY);
    assert_lacks(result, RESOLUTION_POLICY);
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
