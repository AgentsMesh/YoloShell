use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const RESOLUTION_POLICY: &str = "retained authority resolution is indeterminate";
const RETAINED_SOURCE: &str = r#"
struct RetainedExactProcess;
pub struct DurableContainment { target: RetainedExactProcess }
"#;

#[test]
fn literal_include_shares_the_containing_module_scope() {
    let fixture = rooted_fixture();
    fixture.write(
        "core/src/lib.rs",
        "mod authority; use authority::DurableContainment as Hidden; include!(\"included.rs\");\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/included.rs",
        "fn bypass(target: &mut Hidden) { target.enforce_containment(); }\n",
    );
    assert_has(&messages(&fixture), CONSTRUCTOR_POLICY);
}

#[test]
fn inline_module_include_inherits_the_inline_module_path() {
    let fixture = rooted_fixture();
    fixture.write(
        "core/src/lib.rs",
        r#"
mod authority;
mod shell {
    use super::authority::DurableContainment as Hidden;
    include!("shell/included.rs");
}
"#,
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/shell/included.rs",
        "fn bypass(target: &mut Hidden) { target.enforce_containment(); }\n",
    );
    assert_has(&messages(&fixture), CONSTRUCTOR_POLICY);
}

#[test]
fn dynamic_include_path_is_indeterminate() {
    let fixture = rooted_fixture();
    fixture.write(
        "core/src/lib.rs",
        "include!(concat!(\"included\", \".rs\"));\n",
    );
    fixture.write("core/src/included.rs", "pub fn clean() {}\n");
    assert_has(&messages(&fixture), RESOLUTION_POLICY);
}

#[test]
fn local_block_include_is_indeterminate() {
    let fixture = rooted_fixture();
    fixture.write(
        "core/src/lib.rs",
        "fn wrapper() { include!(\"included.rs\"); }\n",
    );
    fixture.write("core/src/included.rs", "pub fn clean() {}\n");
    assert_has(&messages(&fixture), RESOLUTION_POLICY);
}

#[test]
fn self_include_cycle_is_finite_and_indeterminate() {
    let fixture = rooted_fixture();
    fixture.write("core/src/lib.rs", "include!(\"lib.rs\");\n");
    assert_has(&messages(&fixture), RESOLUTION_POLICY);
}

#[test]
fn mutual_include_cycle_is_finite_and_indeterminate() {
    let fixture = rooted_fixture();
    fixture.write("core/src/lib.rs", "include!(\"included.rs\");\n");
    fixture.write("core/src/included.rs", "include!(\"lib.rs\");\n");
    assert_has(&messages(&fixture), RESOLUTION_POLICY);
}

#[test]
fn cfg_test_include_target_is_known_without_inspecting_test_code() {
    let fixture = rooted_fixture();
    fixture.write(
        "core/src/lib.rs",
        "mod authority; use authority::DurableContainment as Hidden; #[cfg(test)] include!(\"tests.rs\");\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/tests.rs",
        "fn test_only(target: &mut Hidden) { target.enforce_containment(); }\n",
    );
    let result = messages(&fixture);
    assert_lacks(&result, RESOLUTION_POLICY);
    assert_lacks(&result, CONSTRUCTOR_POLICY);
}

#[test]
fn active_include_of_a_test_named_file_is_inspected_as_product_code() {
    let fixture = rooted_fixture();
    fixture.write(
        "core/src/lib.rs",
        "mod authority; use authority::DurableContainment as Hidden; include!(\"tests.rs\");\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/tests.rs",
        "fn bypass(target: &mut Hidden) { target.enforce_containment(); }\n",
    );
    assert_has(&messages(&fixture), CONSTRUCTOR_POLICY);
}

#[test]
fn active_path_module_with_a_test_name_is_inspected_as_product_code() {
    let fixture = rooted_fixture();
    fixture.write(
        "core/src/lib.rs",
        "mod authority; use authority::DurableContainment as Hidden; #[path = \"payload_tests.rs\"] mod payload;\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/payload_tests.rs",
        "fn bypass(target: &mut super::Hidden) { target.enforce_containment(); }\n",
    );
    assert_has(&messages(&fixture), CONSTRUCTOR_POLICY);
}

#[test]
fn nested_literal_includes_resolve_from_each_containing_file() {
    let fixture = rooted_fixture();
    fixture.write(
        "core/src/lib.rs",
        "mod authority; use authority::DurableContainment as Hidden; include!(\"parts/a.rs\");\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write("core/src/parts/a.rs", "include!(\"nested/b.rs\");\n");
    fixture.write(
        "core/src/parts/nested/b.rs",
        "fn bypass(target: &mut Hidden) { target.enforce_containment(); }\n",
    );
    assert_has(&messages(&fixture), CONSTRUCTOR_POLICY);
}

fn rooted_fixture() -> Fixture {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/BUILD.bazel",
        "rust_library(name = \"root\", crate_root = \"src/lib.rs\")\n",
    );
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
