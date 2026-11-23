use crate::pty_alias_admission_rules as policy;
use yolo_architecture_test_support::Fixture;

const POLICY_PREFIX: &str = "PTY alias admission";
const GLOBAL_POLICY_TEXT: &str = "only in WorkerState::admit_system_mutation_blocking";
const POLLABLE_POLICY_TEXT: &str = "must remain pollable";
const FUTURE_PATH: &str = "core/crates/runtime/engine/worker/src/future_alias_release.rs";
const TEST_PATH: &str = "core/crates/runtime/engine/worker/src/tests/future_alias_release_tests.rs";
const NEW_PROTECTED_PATHS: &[&str] = &[
    policy::RUNTIME_ALIAS_CLOSURE_PATH,
    policy::RUNTIME_ALIAS_CLOSURE_TASK_PATH,
    policy::SECONDARY_CUSTODY_EXPORT_PATH,
    policy::PTY_PROCESS_TERMINATION_PATH,
];

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .filter(|message| message.contains(POLICY_PREFIX))
        .collect()
}

fn method_call(operation: &str) -> String {
    format!("impl WorkerState {{ fn release(&self) {{ self.gate.{operation}(); }} }}\n")
}

fn protected_operation(path: &str, operation: &str) -> String {
    let operation = method_call(operation);
    if path != policy::PTY_PROCESS_TERMINATION_PATH {
        return operation;
    }
    format!(
        "async fn terminate_pty_processes<P: PtyTerminationPermit>(permit: P) {{
let submitted = owner.submit_termination(force)?;
drop(permit);
submitted.wait().await;
}}
{operation}"
    )
}

#[test]
fn accepts_the_typed_blocking_boundary_and_pollable_try_lock() {
    let fixture = Fixture::new();
    fixture.write(
        policy::CONTROL_ADMISSION_PATH,
        format!(
            "impl WorkerState {{ fn admit_system_mutation_blocking(&self) {{ self.gate.{}(); }} }}\n",
            policy::BLOCKING_LOCK_OWNED
        ),
    );
    for path in policy::PROTECTED_PATHS {
        if *path == policy::RUNTIME_ALIAS_CLOSURE_TASK_PATH {
            continue;
        }
        fixture.write(path, protected_operation(path, policy::TRY_LOCK_OWNED));
    }
    assert!(messages(&fixture).is_empty());
}

#[test]
fn accepts_pollable_closure_and_typed_fifo_driver_admission() {
    let fixture = Fixture::new();
    fixture.write(
        policy::RUNTIME_ALIAS_CLOSURE_PATH,
        method_call(policy::TRY_LOCK_OWNED),
    );
    fixture.write(
        policy::CONTROL_ADMISSION_PATH,
        format!(
            "impl WorkerState {{ async fn admit_alias_closure_mutation(&self) {{ self.gate.{}().await; }} }}\n",
            policy::LOCK_OWNED
        ),
    );
    fixture.write(
        policy::RUNTIME_ALIAS_CLOSURE_TASK_PATH,
        "async fn drive(state: &WorkerState) { state.admit_alias_closure_mutation().await; }\n",
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_starvation_prone_try_lock_polling_in_fifo_driver() {
    let fixture = Fixture::new();
    fixture.write(
        policy::RUNTIME_ALIAS_CLOSURE_TASK_PATH,
        method_call(policy::TRY_LOCK_OWNED),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("typed FIFO mutation admission"));
}

#[test]
fn rejects_blocking_spin_and_block_on_in_each_new_convergence_path() {
    for path in NEW_PROTECTED_PATHS {
        assert!(policy::PROTECTED_PATHS.contains(path));
        for operation in policy::FORBIDDEN_OPERATIONS {
            let fixture = Fixture::new();
            fixture.write(path, protected_operation(path, operation));
            let result = messages(&fixture);
            assert_eq!(result.len(), 1, "path={path} operation={operation}");
        }
    }
}

#[test]
fn rejects_blocking_lock_outside_the_exact_typed_boundary() {
    let fixture = Fixture::new();
    fixture.write(FUTURE_PATH, method_call(policy::BLOCKING_LOCK_OWNED));
    fixture.write(TEST_PATH, method_call(policy::BLOCKING_LOCK_OWNED));
    fixture.write(
        policy::CONTROL_ADMISSION_PATH,
        format!(
            "impl WorkerState {{ fn bypass(&self) {{ self.gate.{}(); }} }}\n",
            policy::BLOCKING_LOCK_OWNED
        ),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 3);
    assert!(
        result
            .iter()
            .all(|message| message.contains(GLOBAL_POLICY_TEXT))
    );
}

#[test]
fn rejects_blocking_lock_under_the_wrong_owner() {
    let fixture = Fixture::new();
    fixture.write(
        policy::CONTROL_ADMISSION_PATH,
        format!(
            "impl FutureState {{ fn admit_system_mutation_blocking(&self) {{ self.gate.{}(); }} }}\n",
            policy::BLOCKING_LOCK_OWNED
        ),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains(GLOBAL_POLICY_TEXT));
}

#[test]
fn rejects_nonpollable_operations_across_the_owner_release_path() {
    for path in policy::PROTECTED_PATHS {
        for operation in policy::FORBIDDEN_OPERATIONS {
            let fixture = Fixture::new();
            fixture.write(path, protected_operation(path, operation));
            let result = messages(&fixture);
            assert_eq!(result.len(), 1, "path={path} operation={operation}");
            let expected = if *operation == policy::BLOCKING_LOCK_OWNED {
                GLOBAL_POLICY_TEXT
            } else {
                POLLABLE_POLICY_TEXT
            };
            assert!(
                result[0].contains(expected),
                "path={path} operation={operation}: {result:#?}"
            );
        }
    }
}

#[test]
fn ignores_forbidden_words_in_comments_and_strings() {
    let fixture = Fixture::new();
    fixture.write(
        policy::RUNTIME_ALIAS_CUSTODY_PATH,
        format!(
            "// {} {}\nconst NOTE: &str = \"{} {}\";\n",
            policy::BLOCKING_LOCK_OWNED,
            policy::YIELD_NOW,
            policy::LOCK_OWNED,
            policy::BLOCK_ON
        ),
    );
    assert!(messages(&fixture).is_empty());
}
