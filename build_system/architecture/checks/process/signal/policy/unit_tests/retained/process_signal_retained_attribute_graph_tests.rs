use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const RESOLUTION_POLICY: &str = "retained authority resolution is indeterminate";
const RETAINED_SOURCE: &str = r#"
struct RetainedExactProcess;
pub struct DurableContainment { target: RetainedExactProcess }
"#;

#[test]
fn cfg_attr_test_path_does_not_replace_the_production_default_path() {
    let fixture = rooted_fixture();
    fixture.write(
        "core/src/lib.rs",
        "#[cfg_attr(test, path = \"clean_tests.rs\")] mod authority; mod consumer;\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/clean_tests.rs",
        "pub struct DurableContainment;\n",
    );
    write_consumer(&fixture, "crate::authority::DurableContainment");
    assert_closed(&messages(&fixture));
}

#[test]
fn multiple_conditional_paths_are_indeterminate() {
    let fixture = rooted_fixture();
    fixture.write(
        "core/src/lib.rs",
        r#"
#[cfg_attr(feature = "clean", path = "clean.rs")]
#[cfg_attr(not(feature = "clean"), path = "retained.rs")]
mod authority;
"#,
    );
    fixture.write("core/src/clean.rs", "pub struct DurableContainment;\n");
    fixture.write("core/src/retained.rs", RETAINED_SOURCE);
    assert_has(&messages(&fixture), RESOLUTION_POLICY);
}

#[test]
fn conditional_cfg_any_does_not_disable_the_production_module() {
    let fixture = rooted_fixture();
    fixture.write(
        "core/src/lib.rs",
        "#[cfg_attr(test, cfg(any()))] mod authority; mod consumer;\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    write_consumer(&fixture, "crate::authority::DurableContainment");
    assert_closed(&messages(&fixture));
}

#[test]
fn macro_generated_out_of_line_module_is_indeterminate() {
    let fixture = rooted_fixture();
    fixture.write(
        "core/src/lib.rs",
        r#"
macro_rules! mount {
    ($name:ident) => { #[path = "authority.rs"] mod $name; }
}
mount!(hidden);
fn bypass(target: &mut hidden::DurableContainment) {
    target.enforce_containment();
}
"#,
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    assert_has(&messages(&fixture), RESOLUTION_POLICY);
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

fn write_consumer(fixture: &Fixture, target: &str) {
    fixture.write(
        "core/src/consumer.rs",
        format!("fn bypass(target: &mut {target}) {{ target.enforce_containment(); }}\n"),
    );
}

fn assert_closed(result: &[String]) {
    assert!(
        result.iter().any(|message| {
            message.contains(CONSTRUCTOR_POLICY) || message.contains(RESOLUTION_POLICY)
        }),
        "{result:#?}",
    );
}

fn assert_has(result: &[String], expected: &str) {
    assert!(
        result.iter().any(|message| message.contains(expected)),
        "{result:#?}"
    );
}
