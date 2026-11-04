use yolo_architecture_test_support::Fixture;

const RUST_SOURCE: &str = "core/src/future_pre_exec_policy.rs";
const POLICY_FRAGMENT: &str = "never inside a CommandExt::pre_exec callback";
const SINGLE_VIOLATION: usize = 1;
const QUALIFIED_AND_UNQUALIFIED_VIOLATIONS: usize = 2;

fn messages(source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    fixture.write(RUST_SOURCE, source);
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .filter(|message| message.contains(POLICY_FRAGMENT))
        .collect()
}

#[test]
fn rejects_qualified_and_unqualified_calls_in_inline_callbacks() {
    let source = r#"
use libc::setrlimit;
fn configure(command: &mut Command) {
    command.pre_exec(|| {
        libc::setrlimit(libc::RLIMIT_NPROC, &limit);
        setrlimit(libc::RLIMIT_NOFILE, &limit);
        Ok(())
    });
}
"#;
    let result = messages(source);
    assert_eq!(
        result.len(),
        QUALIFIED_AND_UNQUALIFIED_VIOLATIONS,
        "{result:#?}"
    );
}

#[test]
fn rejects_move_expression_and_ufcs_callback_forms() {
    for source in [
        "fn run(command: &mut Command) { command.pre_exec(move || setrlimit(kind, &limit)); }",
        "fn run(command: &mut Command) { CommandExt::pre_exec(command, || unsafe { libc::setrlimit(kind, &limit) }); }",
        "fn run(command: &mut Command) { <Command as CommandExt>::pre_exec(command, || { setrlimit(kind, &limit); Ok(()) }); }",
    ] {
        assert_eq!(messages(source).len(), SINGLE_VIOLATION, "{source}");
    }
}

#[test]
fn rejects_bound_closures_and_same_file_function_callbacks() {
    for source in [
        "fn run(command: &mut Command) { let callback = || { setrlimit(kind, &limit); Ok(()) }; command.pre_exec(callback); }",
        "fn callback() -> Result<()> { libc::setrlimit(kind, &limit); Ok(()) } fn run(command: &mut Command) { command.pre_exec(callback); }",
    ] {
        assert_eq!(messages(source).len(), SINGLE_VIOLATION, "{source}");
    }
}

#[test]
fn permits_setrlimit_in_normal_process_context() {
    let source = r#"
fn apply_limit() {
    let result = unsafe { libc::setrlimit(libc::RLIMIT_NPROC, &limit) };
    consume(result);
}
"#;
    assert!(messages(source).is_empty());
}

#[test]
fn permits_async_signal_safe_pre_exec_callbacks() {
    let source = r#"
fn configure(command: &mut Command) {
    command.pre_exec(move || close_descriptor(descriptor));
}
"#;
    assert!(messages(source).is_empty());
}

#[test]
fn permits_a_safe_named_callback_and_an_unrelated_limit_helper() {
    let source = r#"
fn callback() -> Result<()> { close_descriptor(descriptor) }
fn apply_limit() { libc::setrlimit(kind, &limit); }
fn run(command: &mut Command) { command.pre_exec(callback); }
"#;
    assert!(messages(source).is_empty());
}
