use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const RESOLUTION_POLICY: &str = "retained authority resolution is indeterminate";
const RETAINED_SOURCE: &str = r#"
struct RetainedExactProcess;
pub struct DurableContainment { target: RetainedExactProcess }
"#;

#[test]
fn clean_public_alias_cycle_does_not_forge_retained_identity() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        "pub mod left; pub mod right; mod consumer;\n",
    );
    fixture.write(
        "core/src/left.rs",
        "pub struct Clean; pub use crate::right as hop;\n",
    );
    fixture.write("core/src/right.rs", "pub use crate::left as hop;\n");
    fixture.write(
        "core/src/consumer.rs",
        r#"
use crate::left::hop::hop::Clean;
fn allowed(target: &mut Clean) { target.enforce_containment(); }
"#,
    );
    assert_lacks(&messages(&fixture), CONSTRUCTOR_POLICY);
}

#[test]
fn repeated_self_aliases_still_reach_a_finite_retained_path() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        "pub extern crate self as yolo; pub mod authority; mod consumer;\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/consumer.rs",
        r#"
use crate::yolo::yolo::authority::DurableContainment;
fn bypass(target: &mut DurableContainment) { target.begin_containment(); }
"#,
    );
    assert_has(&messages(&fixture), CONSTRUCTOR_POLICY);
}

#[test]
fn component_library_root_owns_the_crate_namespace() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/crates/application/attach/cli/BUILD.bazel",
        r#"
rust_library(
    name = "attach",
    crate_root = "src/lib.rs",
)
"#,
    );
    fixture.write(
        "core/crates/application/attach/cli/src/lib.rs",
        r#"
mod authority {
    struct RetainedExactProcess;
    pub struct DurableContainment { target: RetainedExactProcess }
}
fn bypass(target: &mut crate::authority::DurableContainment) {
    target.enforce_containment();
}
"#,
    );
    assert_has(&messages(&fixture), CONSTRUCTOR_POLICY);
}

#[test]
fn cfg_disabled_self_path_does_not_expand_the_module_graph() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        r#"
#![allow(dead_code)]
#[cfg(any())]
#[path = "lib.rs"]
mod nested;
pub fn clean() {}
"#,
    );
    let result = messages(&fixture);
    assert_lacks(&result, CONSTRUCTOR_POLICY);
    assert_lacks(&result, RESOLUTION_POLICY);
}

#[test]
fn active_self_path_cycle_is_finite_and_indeterminate() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        "#[path = \"lib.rs\"] mod nested;\npub fn clean() {}\n",
    );
    assert_has(&messages(&fixture), RESOLUTION_POLICY);
}

#[test]
fn active_mutual_path_cycle_is_finite_and_indeterminate() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        "#[path = \"a.rs\"] mod a;\npub fn clean() {}\n",
    );
    fixture.write(
        "core/src/a.rs",
        "#[path = \"../lib.rs\"] mod back;\npub fn clean() {}\n",
    );
    assert_has(&messages(&fixture), RESOLUTION_POLICY);
}

#[test]
fn growing_alias_rewrite_budget_is_indeterminate_not_retained() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        r#"
pub mod looping { pub mod nested {} }
pub use crate::looping::nested as looping;
mod consumer;
"#,
    );
    fixture.write(
        "core/src/consumer.rs",
        r#"
fn clean(target: &mut crate::looping::Clean) {
    target.enforce_containment();
}
"#,
    );
    let result = messages(&fixture);
    assert_has(&result, RESOLUTION_POLICY);
    assert_lacks(&result, CONSTRUCTOR_POLICY);
}

#[test]
fn local_alias_growth_is_bounded_and_indeterminate() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        r#"
pub mod authority;
pub extern crate self as yolo;
#[cfg(any())]
mod disabled {
    extern crate self as yolo;
    use yolo::yolo as yolo;
}
"#,
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    assert_has(&messages(&fixture), RESOLUTION_POLICY);
}

#[test]
fn missing_explicit_crate_root_is_indeterminate_without_orphan_fallback() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/BUILD.bazel",
        "rust_library(name = \"missing\", crate_root = \"src/missing.rs\")\n",
    );
    fixture.write("core/src/orphan.rs", "pub fn clean() {}\n");
    assert_has(&messages(&fixture), RESOLUTION_POLICY);
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
