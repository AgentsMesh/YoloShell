use yolo_architecture_test_support::Fixture;

const PTY_SESSION_TEST_SOURCE: &str =
    "core/crates/runtime/engine/process/unit_tests/pty_session_future_tests.rs";
const UNRELATED_TEST_SOURCE: &str =
    "core/crates/application/remote/cli/unit_tests/process_future_tests.rs";
const POLICY_TEXT: &str = "must use the centralized waitable spawn boundary";

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_raw_command_spawn_in_the_pty_session_domain() {
    let fixture = Fixture::new();
    fixture.write(
        PTY_SESSION_TEST_SOURCE,
        "fn launch(mut command: Command) { command.spawn().unwrap(); }\n",
    );

    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains(POLICY_TEXT));
}

#[test]
fn permits_the_centralized_waitable_spawn_helper() {
    let fixture = Fixture::new();
    fixture.write(
        PTY_SESSION_TEST_SOURCE,
        "fn launch(command: Command) { spawn_waitable_test_child(command); }\n",
    );

    assert!(messages(&fixture).is_empty());
}

#[test]
fn ignores_raw_spawn_outside_the_pty_session_domain() {
    let fixture = Fixture::new();
    fixture.write(
        UNRELATED_TEST_SOURCE,
        "fn launch(mut command: Command) { command.spawn().unwrap(); }\n",
    );

    assert!(messages(&fixture).is_empty());
}

#[test]
fn ignores_spawn_text_without_a_method_call() {
    let fixture = Fixture::new();
    fixture.write(
        PTY_SESSION_TEST_SOURCE,
        "const NOTE: &str = \"command.spawn()\";\n// command.spawn()\n",
    );

    assert!(messages(&fixture).is_empty());
}
