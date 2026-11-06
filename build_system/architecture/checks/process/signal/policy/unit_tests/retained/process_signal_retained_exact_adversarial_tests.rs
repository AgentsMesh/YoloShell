use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_process_signal_foundation::process_signal_guardian_constants as guardian;

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_retained_exact.rs";
const TYPE_POLICY: &str = "authority type escaped";

#[test]
fn macro_bodies_and_selected_method_names_cannot_move_operations() {
    for body in [
        r#"
macro_rules! retained_op { ($target:expr) => { $target.begin_containment() } }
struct Other;
fn bypass(target: &mut Other) { retained_op!(target); }
"#,
        r#"
macro_rules! retained_op ( ($target:expr) => { $target.begin_containment() } );
struct Other;
fn bypass(target: &mut Other) { retained_op!(target); }
"#,
        r#"
macro_rules! retained_op [ ($target:expr) => { $target.begin_containment() } ];
struct Other;
fn bypass(target: &mut Other) { retained_op!(target); }
"#,
    ] {
        assert_fixture(FUTURE_PATH, body, 1, 0);
    }

    let selected = r#"
macro_rules! retained_op { ($target:expr, $method:ident) => { $target.$method() } }
struct Other;
fn bypass(target: &mut Other) { retained_op!(target, begin_containment); }
"#;
    assert_fixture(FUTURE_PATH, selected, 1, 0);
}

#[test]
fn unrelated_guardian_receivers_are_not_retained_authorities() {
    let source = format!(
        "struct Other; impl {} {{ fn {}(metrics: &mut Other) {{ \
         metrics.begin_containment(); metrics.observe(); }} }}\n",
        guardian::TARGET_OWNER,
        guardian::GRACEFUL_FUNCTION,
    );
    assert_fixture(guardian::PATH, &source, 0, 0);
}

#[test]
fn propagated_bindings_are_checked_outside_type_allowlisted_paths() {
    let source = r#"
fn bypass(authority: &mut RetainedExactProcess) {
    let target = authority;
    target.begin_containment();
}
"#;
    assert_fixture(FUTURE_PATH, source, 1, 1);
}

#[test]
fn retained_self_receivers_remain_bound_to_exact_methods() {
    let source = r#"
impl RetainedExactProcess {
    fn bypass(&mut self) { self.begin_containment(); }
}
"#;
    assert_fixture(guardian::PATH, source, 1, 0);
}

#[test]
fn uninitialized_lets_do_not_capture_later_assignments() {
    let source = r#"
struct Other;
fn allowed(other: &mut Other, authority: &mut RetainedExactProcess) {
    let innocent;
    let retained = authority;
    innocent = other;
    innocent.begin_containment();
    consume(retained);
}
"#;
    assert_fixture(FUTURE_PATH, source, 0, 1);
}

#[test]
fn retained_struct_destructuring_propagates_renamed_fields() {
    let source = r#"
struct Wrapper { target: RetainedExactProcess }
fn bypass(wrapper: Wrapper) {
    let Wrapper { target: moved } = wrapper;
    moved.begin_containment();
}
"#;
    assert_fixture(FUTURE_PATH, source, 1, 1);
}

#[test]
fn retained_tuple_structs_and_enums_propagate_authority() {
    let tuple = r#"
struct Wrapper(RetainedExactProcess);
fn bypass(wrapper: Wrapper) {
    let Wrapper(target) = wrapper;
    target.begin_containment();
}
"#;
    assert_fixture(guardian::PATH, tuple, 1, 0);

    let enumeration = r#"
enum Wrapper { Target(RetainedExactProcess) }
fn bypass(wrapper: Wrapper) {
    let Wrapper::Target(target) = wrapper else { return; };
    target.enforce_containment();
}
"#;
    assert_fixture(guardian::PATH, enumeration, 1, 0);

    let named = r#"
struct Wrapper<T> where T: Fn() { target: RetainedExactProcess, callback: T }
fn bypass(wrapper: Wrapper<fn()>) { wrapper.target.begin_containment(); }
"#;
    assert_fixture(guardian::PATH, named, 1, 0);
}

#[test]
fn nested_generic_and_tuple_types_propagate_retained_bindings() {
    let generic = r#"
fn bypass(target: &mut Wrapper<Other, RetainedExactProcess>) {
    target.begin_containment();
}
"#;
    assert_fixture(FUTURE_PATH, generic, 1, 1);

    let tuple = r#"
fn bypass(target: &mut (Other, RetainedExactProcess)) {
    target.enforce_containment();
}
"#;
    assert_fixture(FUTURE_PATH, tuple, 1, 1);
}

#[test]
fn dereference_and_adapter_receivers_cannot_hide_operations() {
    let dereference = r#"
fn bypass(target: &mut RetainedExactProcess) {
    (*target).begin_containment();
}
"#;
    assert_fixture(FUTURE_PATH, dereference, 1, 1);

    let adapter = r#"
fn bypass(target: &mut Option<RetainedExactProcess>) {
    target.as_mut().unwrap().enforce_containment();
}
"#;
    assert_fixture(FUTURE_PATH, adapter, 1, 1);
}

fn assert_fixture(path: &str, source: &str, constructors: usize, types: usize) {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(path, source);
    let result = messages(&fixture);
    assert_eq!(
        matches(&result, CONSTRUCTOR_POLICY),
        constructors,
        "{result:#?}"
    );
    assert_eq!(matches(&result, TYPE_POLICY), types, "{result:#?}");
}

fn matches(messages: &[String], expected: &str) -> usize {
    messages
        .iter()
        .filter(|message| message.contains(expected))
        .count()
}
