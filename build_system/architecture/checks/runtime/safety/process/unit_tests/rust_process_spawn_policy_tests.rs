use yolo_architecture_test_support::Fixture;

const RUNTIME_TEST_SOURCE: &str =
    "core/crates/runtime/engine/worker/unit_tests/future_process_tests.rs";
const INDEPENDENT_TEST_SOURCE: &str =
    "core/crates/application/remote/cli/unit_tests/future_process_tests.rs";
const POLICY_TEXT: &str = "must enter the waitable spawn gate";

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_direct_chained_and_bound_command_spawns() {
    let fixture = Fixture::new();
    fixture.write(
        RUNTIME_TEST_SOURCE,
        "use std::process::Command;\nfn direct() { Command::new(\"true\").spawn(); }\nfn bound() { let mut child_command = Command::new(\"true\"); child_command.spawn(); }\n",
    );

    let result = messages(&fixture);
    assert_eq!(result.len(), 2);
    assert!(result.iter().all(|message| message.contains(POLICY_TEXT)));
}

#[test]
fn rejects_command_parameters_and_factory_results() {
    let fixture = Fixture::new();
    fixture.write(
        RUNTIME_TEST_SOURCE,
        "use std::process::Command;\nfn helper_command() -> Command { Command::new(\"true\") }\nfn parameter(command: &mut Command) { command.spawn(); }\nfn factory() { helper_command().spawn(); }\n",
    );

    let result = messages(&fixture);
    assert_eq!(result.len(), 2);
}

#[test]
fn permits_a_guard_acquired_before_the_process_spawn() {
    let fixture = Fixture::new();
    fixture.write(
        RUNTIME_TEST_SOURCE,
        "use std::process::Command;\nfn launch() { let _guard = waitable_child_spawn_guard(); Command::new(\"true\").spawn(); }\n",
    );

    assert!(messages(&fixture).is_empty());
}

#[test]
fn permits_a_deadline_guard_acquired_before_the_process_spawn() {
    let fixture = Fixture::new();
    fixture.write(
        RUNTIME_TEST_SOURCE,
        "use std::process::Command;\nfn launch(command: &mut Command) { let _guard = waitable_child_spawn_guard_before(deadline); command.spawn(); }\n",
    );

    assert!(messages(&fixture).is_empty());
}

#[test]
fn permits_spawn_inside_the_typed_waitable_boundary() {
    let fixture = Fixture::new();
    fixture.write(
        crate::rust_process_spawn_contract::SPAWN_BOUNDARY_SOURCE,
        "use std::process::Command;\nstruct WaitableChildSpawnBoundary;\nimpl WaitableChildSpawnBoundary { fn spawn_command(&self, command: &mut Command) { command.spawn(); } }\n",
    );

    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_spawn_inside_an_unrelated_boundary_type() {
    let fixture = Fixture::new();
    fixture.write(
        crate::rust_process_spawn_contract::SPAWN_BOUNDARY_SOURCE,
        "use std::process::Command;\nstruct OtherBoundary;\nimpl OtherBoundary { fn spawn_command(&self, command: &mut Command) { command.spawn(); } }\n",
    );

    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
}

#[test]
fn rejects_a_boundary_name_outside_the_authority_source() {
    let fixture = Fixture::new();
    fixture.write(
        RUNTIME_TEST_SOURCE,
        "use std::process::Command;\nstruct WaitableChildSpawnBoundary;\nimpl WaitableChildSpawnBoundary { fn spawn_command(&self, command: &mut Command) { command.spawn(); } }\n",
    );

    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
}

#[test]
fn comments_strings_and_late_guards_cannot_forge_the_boundary() {
    let fixture = Fixture::new();
    fixture.write(
        RUNTIME_TEST_SOURCE,
        "use std::process::Command;\nfn launch() { /* waitable_child_spawn_guard() */ let note = \"waitable_child_spawn_guard()\"; Command::new(\"true\").spawn(); let _guard = waitable_child_spawn_guard(); }\n",
    );

    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains(POLICY_TEXT));
}

#[test]
fn ignores_nonprocess_spawns_and_independent_test_binaries() {
    let fixture = Fixture::new();
    fixture.write(
        RUNTIME_TEST_SOURCE,
        "fn launch(tasks: Tasks) { tasks.spawn(); }\n",
    );
    fixture.write(
        INDEPENDENT_TEST_SOURCE,
        "fn launch() { std::process::Command::new(\"true\").spawn(); }\n",
    );

    assert!(messages(&fixture).is_empty());
}
