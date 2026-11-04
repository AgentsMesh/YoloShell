use super::*;

#[test]
fn child_spawn_enters_the_gate_before_spawning() {
    let rejection = super::super::reject_spawn_gate_entry_for_test();
    let command = Command::new(TEST_EXIT_EXECUTABLE);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        spawn_waitable_test_child(command)
    }));
    drop(rejection);
    if let Ok(mut child) = outcome {
        let _ = child.wait();
        panic!("{MISSING_GATE_DETAIL}");
    }
}

#[test]
fn existing_boundary_spawn_does_not_reenter_the_gate() {
    let guard = super::super::waitable_child_spawn_guard().unwrap();
    let rejection = super::super::reject_spawn_gate_entry_for_test();
    let mut child = spawn_waitable_test_child_in_boundary(
        Command::new(TEST_EXIT_EXECUTABLE),
        &guard.boundary(),
    );
    drop(rejection);
    drop(guard);
    assert!(child.wait().unwrap().success());
}
