use yolo_architecture_test_support::Fixture;

const RUST_SOURCE: &str = "core/src/future_pre_exec_adversarial.rs";
const POLICY_FRAGMENT: &str = "never inside a CommandExt::pre_exec callback";
const SINGLE_VIOLATION: usize = 1;

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
fn nested_delimiters_and_typed_closure_parameters_cannot_hide_the_call() {
    let source = r#"
fn run(command: &mut Command) {
    command.pre_exec((move |first: (u32, u32), second: [u32; 2]| -> Result<()> {
        inspect((first, second));
        unsafe { crate::platform::setrlimit(kind, &limit) };
        Ok(())
    }));
}
"#;
    assert_eq!(messages(source).len(), SINGLE_VIOLATION);
}

#[test]
fn raw_identifiers_cannot_bypass_the_exact_call_rule() {
    let source =
        "fn run(command: &mut Command) { command.pre_exec(|| r#setrlimit(kind, &limit)); }";
    assert_eq!(messages(source).len(), SINGLE_VIOLATION);
}

#[test]
fn parenthesized_function_paths_cannot_hide_the_call() {
    let source = r#"
fn run(command: &mut Command) {
    command.pre_exec(|| unsafe { ((libc::setrlimit))(kind, &limit) });
}
"#;
    assert_eq!(messages(source).len(), SINGLE_VIOLATION);
}

#[test]
fn control_flow_after_an_earlier_block_cannot_hide_the_call() {
    let source = r#"
fn run(command: &mut Command) {
    command.pre_exec(|| if ready() {
        Ok(())
    } else {
        setrlimit(kind, &limit);
        Ok(())
    });
}
"#;
    assert_eq!(messages(source).len(), SINGLE_VIOLATION);
}

#[test]
fn comments_strings_and_nearby_names_do_not_forge_a_call() {
    let source = r#"
fn run(command: &mut Command) {
    // command.pre_exec(|| libc::setrlimit(kind, &limit));
    let evidence = "pre_exec(|| setrlimit(kind, &limit))";
    command.pre_exec(|| setrlimit_checked(kind, &limit));
    consume(evidence);
}
"#;
    assert!(messages(source).is_empty());
}

#[test]
fn similarly_named_or_unqualified_functions_are_not_commandext_calls() {
    let source = r#"
fn run(command: &mut Command) {
    command.pre_execution(|| setrlimit(kind, &limit));
    pre_exec(|| setrlimit(kind, &limit));
}
"#;
    assert!(messages(source).is_empty());
}

#[test]
fn a_call_before_or_after_the_callback_is_not_inside_it() {
    let source = r#"
fn run(command: &mut Command) {
    setrlimit(kind, &limit);
    command.pre_exec(|| Ok(()));
    libc::setrlimit(kind, &limit);
}
"#;
    assert!(messages(source).is_empty());
}
