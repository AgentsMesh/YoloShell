use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_policy_contracts::process_signal as contract;

const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_retained_visibility.rs";
const SENSITIVE_PATH: &str = contract::EXACT_TERMINATION_PATH;

#[test]
fn visible_functions_cannot_accept_retained_authority() {
    #[rustfmt::skip]
    let visible = ["pub", "pub(super)", "pub(crate)", "pub(in super)", "pub(in crate)"];
    for visibility in visible {
        let source = format!(
            "struct RetainedExactProcess;\n\
             {visibility} fn leak(authority: RetainedExactProcess) {{ drop(authority); }}"
        );
        assert_type_violation(SENSITIVE_PATH, &source);
    }
}

#[test]
fn visible_constants_and_statics_cannot_export_retained_authority() {
    for declaration in [
        "pub const LEAK: Option<RetainedExactProcess> = None;",
        "pub(super) const LEAK: Option<RetainedExactProcess> = None;",
        "pub(crate) static LEAK: Option<RetainedExactProcess> = None;",
        "pub(in super) static LEAK: Option<RetainedExactProcess> = None;",
        "pub(in crate) static mut LEAK: Option<RetainedExactProcess> = None;",
    ] {
        let source = format!("struct RetainedExactProcess;\n{declaration}");
        assert_type_violation(SENSITIVE_PATH, &source);
    }
}

#[test]
fn associated_types_cannot_create_authority_adapters() {
    let inherent = r#"
#![feature(inherent_associated_types)]
struct RetainedExactProcess;
struct Wrapper;
impl Wrapper { pub(crate) type Authority = RetainedExactProcess; }
"#;
    assert_type_violation(SENSITIVE_PATH, inherent);

    let trait_implementation = r#"
struct RetainedExactProcess;
pub trait AuthorityView { type Authority; }
pub struct Wrapper;
impl AuthorityView for Wrapper { type Authority = RetainedExactProcess; }
"#;
    assert_type_violation(SENSITIVE_PATH, trait_implementation);
}

#[test]
fn inherent_hrtb_bounds_do_not_look_like_trait_separators() {
    let source = r#"
#![feature(inherent_associated_types)]
struct RetainedExactProcess;
struct Wrapper<T>(T);
impl<T> Wrapper<T> where T: for<'a> Fn(&'a str) {
    type PrivateAuthority = RetainedExactProcess;
}
"#;
    assert_type_clean(SENSITIVE_PATH, source);
}

#[test]
fn retained_macro_definitions_fail_closed() {
    for source in [
        r#"
macro_rules! leak {
    ($vis:vis, $authority:ty) => { $vis type Authority = $authority; }
}
"#,
        r#"
macro_rules! leak {
    () => { pub(crate) fn leak(authority: RetainedExactProcess) {} }
}
"#,
        r#"
macro_rules! leak {
    ($authority:ty) => { pub(crate) type Authority = $authority; }
}
"#,
    ] {
        assert_type_violation(SENSITIVE_PATH, source);
    }
}

#[test]
fn provably_inert_public_macros_remain_allowed() {
    let source = r#"
macro_rules! wrapper { ($name:ident) => { pub(crate) struct $name; } }
macro_rules! visible_wrapper { ($vis:vis $name:ident) => { $vis struct $name; } }
macro_rules! methods {
    ($state:ty) => { impl $state { pub fn count(&self) -> usize { 0 } } }
}
struct Example;
wrapper!(Wrapper);
visible_wrapper!(pub(crate) VisibleWrapper);
methods!(Example);
"#;
    assert_type_clean(FUTURE_PATH, source);
}

#[test]
fn module_private_visibility_does_not_export_authority() {
    let source = r#"
struct RetainedExactProcess;
pub(self) fn consume(authority: RetainedExactProcess) { drop(authority); }
pub(in self) const PRIVATE_AUTHORITY: Option<RetainedExactProcess> = None;
struct Wrapper { pub(self) authority: RetainedExactProcess }
"#;
    assert_type_clean(SENSITIVE_PATH, source);
}

fn inspect(path: &str, source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(path, source);
    messages(&fixture)
}

fn assert_type_violation(path: &str, source: &str) {
    assert_has(&inspect(path, source), contract::AUTHORITY_TYPE_MESSAGE);
}

fn assert_type_clean(path: &str, source: &str) {
    assert_lacks(&inspect(path, source), contract::AUTHORITY_TYPE_MESSAGE);
}

#[rustfmt::skip]
fn assert_has(messages: &[String], expected: &str) {
    assert!(messages.iter().any(|message| message.contains(expected)), "{messages:#?}");
}

#[rustfmt::skip]
fn assert_lacks(messages: &[String], expected: &str) {
    assert!(!messages.iter().any(|message| message.contains(expected)), "{messages:#?}");
}
