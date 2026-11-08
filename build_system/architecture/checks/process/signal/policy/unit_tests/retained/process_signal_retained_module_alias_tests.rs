use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_process_signal_foundation::process_signal_guardian_constants as guardian;

const CONSUMER_PATH: &str = "core/src/process/future_retained_alias.rs";
const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const OTHER_PATH: &str = "core/src/other/containment.rs";
const RETAINED_PATH: &str = guardian::PATH;
const EXPECTED_SENSITIVE_OPERATIONS: usize = 1;

#[test]
fn local_module_and_crate_aliases_preserve_retained_identity() {
    for source in [
        r#"
        use crate::settlement_containment as hidden;
fn bypass(target: &mut hidden::DurableContainment) {
    target.enforce_containment();
}
"#,
        r#"
extern crate self as hidden;
fn bypass(target: &mut hidden::settlement_containment::DurableContainment) {
    target.begin_containment();
}
"#,
    ] {
        assert_has(&consumer_messages(source));
    }
}

#[test]
fn sibling_super_module_aliases_preserve_retained_identity() {
    let fixture = retained_fixture();
    fixture.write(
        "core/crates/runtime/engine/custody/guardian/src/future_alias.rs",
        r#"
use super::settlement_containment as hidden;
fn bypass(target: &mut hidden::DurableContainment) {
    target.enforce_containment();
}
"#,
    );
    assert_has(&messages(&fixture));
}

#[test]
fn path_mapped_modules_preserve_exported_retained_identity() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        "#[path = \"authority_impl.rs\"]\npub mod hidden;\n",
    );
    fixture.write(
        "core/src/authority_impl.rs",
        r#"
struct RetainedExactProcess;
pub struct DurableContainment { target: RetainedExactProcess }
"#,
    );
    fixture.write(
        CONSUMER_PATH,
        r#"
use crate::hidden::DurableContainment;
fn bypass(target: &mut DurableContainment) { target.enforce_containment(); }
"#,
    );
    assert_has(&messages(&fixture));
}

#[test]
fn public_module_reexports_preserve_retained_identity() {
    for bridge in [
        "pub use crate::settlement_containment as hidden;\n",
        r#"
pub mod hidden {
    pub use crate::settlement_containment::DurableContainment;
}
"#,
    ] {
        let fixture = retained_fixture();
        fixture.write("core/src/bridge.rs", bridge);
        fixture.write(
            CONSUMER_PATH,
            r#"
use crate::bridge::hidden::DurableContainment;
fn bypass(target: &mut DurableContainment) { target.begin_containment(); }
"#,
        );
        assert_has(&messages(&fixture));
    }
}

#[test]
fn module_aliases_feed_grouped_glob_imports() {
    let result = consumer_messages(
        r#"
        use crate::settlement_containment as hidden;
use hidden::*;
fn bypass(target: &mut DurableContainment) { target.enforce_containment(); }
"#,
    );
    assert_has(&result);
}

#[test]
fn inner_unrelated_aliases_shadow_sensitive_parent_aliases() {
    let fixture = retained_fixture();
    fixture.write(OTHER_PATH, "pub struct DurableContainment;\n");
    fixture.write(
        CONSUMER_PATH,
        r#"
mod retained {
        use crate::settlement_containment as hidden;
    fn bypass(target: &mut hidden::DurableContainment) {
        target.enforce_containment();
    }
}
mod clean {
    use crate::other::containment as hidden;
    fn allowed(target: &mut hidden::DurableContainment) {
        target.enforce_containment();
    }
}
"#,
    );
    let result = messages(&fixture);
    assert_eq!(
        result
            .iter()
            .filter(|message| message.contains(CONSTRUCTOR_POLICY))
            .count(),
        EXPECTED_SENSITIVE_OPERATIONS,
        "{result:#?}"
    );
}

#[test]
fn unrelated_module_aliases_remain_clean() {
    let fixture = retained_fixture();
    fixture.write(OTHER_PATH, "pub struct DurableContainment;\n");
    fixture.write(
        CONSUMER_PATH,
        r#"
use crate::other::containment as hidden;
fn allowed(target: &mut hidden::DurableContainment) {
    target.begin_containment();
}
"#,
    );
    assert!(
        messages(&fixture)
            .iter()
            .all(|message| !message.contains(CONSTRUCTOR_POLICY))
    );
}

fn consumer_messages(source: &str) -> Vec<String> {
    let fixture = retained_fixture();
    fixture.write(CONSUMER_PATH, source);
    messages(&fixture)
}

fn retained_fixture() -> Fixture {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        RETAINED_PATH,
        r#"
struct RetainedExactProcess;
pub(super) struct DurableContainment { target: RetainedExactProcess }
"#,
    );
    fixture
}

fn assert_has(result: &[String]) {
    assert!(
        result
            .iter()
            .any(|message| message.contains(CONSTRUCTOR_POLICY)),
        "{result:#?}"
    );
}
