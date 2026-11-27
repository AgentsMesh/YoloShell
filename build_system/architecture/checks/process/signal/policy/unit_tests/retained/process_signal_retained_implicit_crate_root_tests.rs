use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const RESOLUTION_POLICY: &str = "retained authority resolution is indeterminate";
const RETAINED_ROOT: &str = r#"
struct RetainedExactProcess;
struct DurableContainment { target: RetainedExactProcess }
fn bypass(target: &mut DurableContainment) { target.enforce_containment(); }
"#;

#[test]
fn single_source_implicit_root_is_scanned_beside_an_explicit_root() {
    let fixture = fixture_with_build(
        r#"
rust_library(name = "explicit", crate_root = "src/clean.rs")
rust_binary(name = "retained", srcs = ["src/retained_main.rs"])
"#,
    );
    fixture.write("core/src/clean.rs", "pub fn clean() {}\n");
    fixture.write("core/src/retained_main.rs", RETAINED_ROOT);
    let result = messages(&fixture);
    assert_has(&result, CONSTRUCTOR_POLICY);
    assert_lacks(&result, RESOLUTION_POLICY);
}

#[test]
fn named_implicit_root_is_selected_from_multiple_sources() {
    let fixture = fixture_with_build(
        r#"
rust_library(name = "explicit", crate_root = "src/clean.rs")
rust_binary(
    name = "retained",
    srcs = ["src/support.rs", "src/retained.rs"],
)
"#,
    );
    fixture.write("core/src/clean.rs", "pub fn clean() {}\n");
    fixture.write("core/src/support.rs", "pub fn support() {}\n");
    fixture.write("core/src/retained.rs", RETAINED_ROOT);
    let result = messages(&fixture);
    assert_has(&result, CONSTRUCTOR_POLICY);
    assert_lacks(&result, RESOLUTION_POLICY);
}

#[test]
fn dynamic_implicit_root_fails_closed_without_orphan_scanning() {
    let fixture = fixture_with_build(
        r#"
rust_library(name = "explicit", crate_root = "src/clean.rs")
rust_binary(name = "retained", srcs = glob(["src/*.rs"]))
"#,
    );
    fixture.write("core/src/clean.rs", "pub fn clean() {}\n");
    fixture.write("core/src/retained.rs", "mod authority; mod consumer;\n");
    fixture.write(
        "core/src/retained/authority.rs",
        "struct RetainedExactProcess; pub struct DurableContainment { target: RetainedExactProcess }\n",
    );
    fixture.write(
        "core/src/retained/consumer.rs",
        r#"
fn bypass(target: &mut crate::authority::DurableContainment) {
    target.enforce_containment();
}
"#,
    );
    let result = messages(&fixture);
    assert_has(&result, RESOLUTION_POLICY);
    assert_lacks(&result, CONSTRUCTOR_POLICY);
}

fn fixture_with_build(build: &str) -> Fixture {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write("core/BUILD.bazel", build);
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
