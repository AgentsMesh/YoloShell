use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_guardian_constants as guardian;

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const FUTURE_PATH: &str = "core/src/process/future_retained_exact.rs";
const SECOND_FUTURE_PATH: &str = "core/src/process/future_retained_consumer.rs";
const RETAINED_PATH: &str = guardian::PATH;
const TYPE_POLICY: &str = "authority type escaped";

#[test]
fn deref_and_deref_mut_cannot_create_transparent_authority_adapters() {
    for source in [
        r#"
struct RetainedExactProcess;
struct Wrapper<T>(T);
impl<T> std::ops::Deref for crate::Wrapper<T> {
    type Target = RetainedExactProcess;
}
"#,
        r#"
use std::ops::Deref as AuthorityView;
struct RetainedExactProcess;
struct Wrapper;
impl AuthorityView for Wrapper { type Target = RetainedExactProcess; }
"#,
        r#"
struct RetainedExactProcess;
struct Wrapper(RetainedExactProcess);
impl std::ops::DerefMut for Wrapper {}
"#,
    ] {
        assert_has(
            &single_file(contract::EXACT_TERMINATION_PATH, source),
            TYPE_POLICY,
        );
    }
}

#[test]
fn public_wrappers_fields_and_returns_cannot_export_authority() {
    for source in [
        r#"
struct RetainedExactProcess;
pub struct Opaque { inner: RetainedExactProcess }
"#,
        r#"
struct RetainedExactProcess;
struct Named { pub target: RetainedExactProcess }
struct Tuple(pub RetainedExactProcess);
"#,
        r#"
struct RetainedExactProcess;
struct Private { inner: RetainedExactProcess }
pub fn direct() -> RetainedExactProcess { todo!() }
pub fn opaque() -> Private { todo!() }
"#,
        r#"
struct RetainedExactProcess;
struct Private(RetainedExactProcess);
pub trait Leaking { type Target: Into<RetainedExactProcess>; fn leak() -> Private; }
"#,
    ] {
        assert_has(
            &single_file(contract::EXACT_TERMINATION_PATH, source),
            TYPE_POLICY,
        );
    }
}

#[test]
fn exported_wrappers_taint_cross_file_method_receivers() {
    let direct = consumer_messages(
        r#"
use crate::settlement_containment::DurableContainment;
fn bypass(target: &mut DurableContainment) {
    (&mut target).begin_containment();
}
"#,
    );
    assert_has(&direct, CONSTRUCTOR_POLICY);

    let alias = consumer_messages(
        r#"
use crate::settlement_containment::DurableContainment as Authority;
fn bypass(target: &mut Authority) { target.enforce_containment(); }
"#,
    );
    assert_has(&alias, CONSTRUCTOR_POLICY);
}

#[test]
fn exported_wrapper_catalog_reaches_an_opaque_second_layer() {
    let fixture = retained_fixture();
    fixture.write(
        FUTURE_PATH,
        r#"
use crate::settlement_containment::DurableContainment;
pub struct Outer { inner: DurableContainment }
"#,
    );
    fixture.write(
        SECOND_FUTURE_PATH,
        r#"
use crate::process::future_retained_exact::Outer;
fn bypass(target: &mut Outer) { target.begin_containment(); }
"#,
    );
    let result = retained_messages(&fixture);
    assert_has(&result, TYPE_POLICY);
    assert_has(&result, CONSTRUCTOR_POLICY);
}

#[test]
fn local_same_named_types_shadow_the_exported_catalog() {
    let result = consumer_messages(
        r#"
struct DurableContainment;
fn allowed(target: &mut DurableContainment) {
    target.begin_containment();
    target.observe();
}
"#,
    );
    assert_lacks(&result, CONSTRUCTOR_POLICY);
    assert_lacks(&result, TYPE_POLICY);
}

fn single_file(path: &str, source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(path, source);
    messages(&fixture)
}

fn consumer_messages(source: &str) -> Vec<String> {
    let fixture = retained_fixture();
    fixture.write(FUTURE_PATH, source);
    retained_messages(&fixture)
}

fn retained_messages(fixture: &Fixture) -> Vec<String> {
    messages(fixture)
        .into_iter()
        .filter(|message| !message.starts_with(RETAINED_PATH))
        .collect()
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

fn assert_has(messages: &[String], policy: &str) {
    assert!(
        messages.iter().any(|message| message.contains(policy)),
        "{messages:#?}"
    );
}

fn assert_lacks(messages: &[String], policy: &str) {
    assert!(
        !messages.iter().any(|message| message.contains(policy)),
        "{messages:#?}"
    );
}
