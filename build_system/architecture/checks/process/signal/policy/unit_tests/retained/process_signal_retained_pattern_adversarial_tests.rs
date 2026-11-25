use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_policy_contracts::process_signal as contract;

const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_retained_patterns.rs";

#[test]
fn qualified_inherent_impls_preserve_the_canonical_owner() {
    for source in [
        r#"struct RetainedExactProcess;
impl crate::RetainedExactProcess {
    fn enforce_containment(&mut self) {}
    fn bypass(&mut self) { self.enforce_containment(); }
}"#,
        r#"struct RetainedExactProcess;
impl self::RetainedExactProcess {
    fn enforce_containment(&mut self) {}
    fn bypass(&mut self) { self.enforce_containment(); }
}"#,
        r#"struct RetainedExactProcess;
mod nested { impl super::RetainedExactProcess {
    fn enforce_containment(&mut self) {}
    fn bypass(&mut self) { self.enforce_containment(); }
} }"#,
        r#"extern crate self as yolo;
struct RetainedExactProcess;
impl ::yolo::RetainedExactProcess {
    fn enforce_containment(&mut self) {}
    fn bypass(&mut self) { self.enforce_containment(); }
}"#,
    ] {
        assert_rejected(source);
    }
}

#[test]
fn qualified_unrelated_impls_do_not_taint_self() {
    assert_allowed(
        r#"struct Other;
impl crate::Other {
    fn enforce_containment(&mut self) {}
    fn allowed(&mut self) { self.enforce_containment(); }
}"#,
    );
}

#[test]
fn impl_where_bounds_cannot_replace_the_canonical_owner() {
    assert_rejected(
        r#"struct RetainedExactProcess<F>(F);
impl<F> crate::RetainedExactProcess<F>
where F: for<'a> Fn(&'a ()) {
    fn enforce_containment(&mut self) {}
    fn bypass(&mut self) { self.enforce_containment(); }
}"#,
    );
    assert_rejected(
        r#"trait Operation { fn bypass(&mut self); }
struct RetainedExactProcess;
impl RetainedExactProcess { fn enforce_containment(&mut self) {} }
impl crate::Operation for crate::RetainedExactProcess
where Self: Sized {
    fn bypass(&mut self) { self.enforce_containment(); }
}"#,
    );
}

#[test]
fn for_and_destructuring_assignment_patterns_propagate_authority() {
    assert_rejected(
        r#"fn bypass(targets: Vec<(RetainedExactProcess, ())>) {
    for (mut target, _) in targets { target.enforce_containment(); }
}"#,
    );
    assert_rejected(
        r#"fn bypass(authority: RetainedExactProcess) {
    let mut target;
    (target, _) = (authority, ());
    target.begin_containment();
}"#,
    );
}

#[test]
fn control_expression_blocks_do_not_hide_pattern_sources() {
    assert_rejected(
        r#"fn bypass(targets: Vec<RetainedExactProcess>) {
    for mut target in { targets } { target.enforce_containment(); }
}"#,
    );
    assert_rejected(
        r#"struct Wrapper { authority: RetainedExactProcess }
fn bypass(authority: RetainedExactProcess) {
    match (Wrapper { authority }) {
        Wrapper { authority: ref mut target } => target.begin_containment(),
    }
}"#,
    );
}

#[test]
fn nested_match_and_or_patterns_propagate_authority() {
    assert_rejected(
        r#"enum Either {
    Left(RetainedExactProcess), Right(RetainedExactProcess),
}
fn bypass(mut source: Either) {
    match source {
        Either::Left(ref mut target) | Either::Right(ref mut target) => {
            target.enforce_containment();
        }
    }
}"#,
    );
}

#[test]
fn conditional_let_patterns_propagate_authority() {
    assert_rejected(
        r#"struct Wrapper { authority: RetainedExactProcess }
fn bypass(wrapper: Option<Wrapper>) {
    if let Some(Wrapper { authority: ref mut target }) = wrapper {
        target.begin_containment();
    }
}"#,
    );
    assert_rejected(
        r#"fn bypass(mut source: Option<RetainedExactProcess>) {
    while let Some(ref mut target) = source {
        target.enforce_containment();
        break;
    }
}"#,
    );
}

#[test]
fn closure_parameter_patterns_propagate_invocation_authority() {
    assert_rejected(
        r#"fn bypass(authority: RetainedExactProcess) {
    (|mut target| target.enforce_containment())(authority);
}"#,
    );
    assert_rejected(
        r#"fn bypass(authorities: Vec<RetainedExactProcess>) {
    authorities.into_iter().for_each(|mut target| {
        target.begin_containment();
    });
}"#,
    );
}

#[test]
fn unrelated_pattern_sources_remain_untainted() {
    assert_allowed(
        r#"struct Other;
fn allowed(authority: RetainedExactProcess, others: Vec<Other>) {
    for mut target in others { target.enforce_containment(); }
    let callback = |mut target: Other| target.begin_containment();
    callback(Other);
    consume(authority);
}"#,
    );
}

#[test]
fn unrelated_conditional_sources_remain_untainted() {
    assert_allowed(
        r#"struct Other;
fn allowed(authority: RetainedExactProcess, source: Option<Other>) {
    if let Some(mut target) = source { target.enforce_containment(); }
    consume(authority);
}"#,
    );
}

fn assert_rejected(source: &str) {
    let result = inspect(source);
    assert!(has_policy(&result), "{result:#?}");
}

fn assert_allowed(source: &str) {
    let result = inspect(source);
    assert!(!has_policy(&result), "{result:#?}");
}

fn inspect(source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(FUTURE_PATH, source);
    messages(&fixture)
}

fn has_policy(messages: &[String]) -> bool {
    messages
        .iter()
        .any(|message| message.contains(contract::AUTHORITY_CONSTRUCTOR_MESSAGE))
}
