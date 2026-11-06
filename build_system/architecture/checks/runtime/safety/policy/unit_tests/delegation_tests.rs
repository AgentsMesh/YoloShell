use std::path::PathBuf;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;

const BOUNDARY_PATH: &str = "core/crates/runtime/engine/process/src/linux_containment_future.rs";
const PROCESS_PATH: &str = "core/crates/transport/remote/broker/src/future.rs";
const PTY_PATH: &str = "core/crates/runtime/engine/worker/src/pty/alias/pty_runtime_alias_closure_task.rs";
const PTY_SETTLEMENT_PATH: &str =
    "core/crates/runtime/engine/process/src/pty/pty_master_release_settlement.rs";
const SOCKET_REGISTRATION_PATH: &str =
    "core/crates/persistence/paths/src/socket/namespace/socket_namespace_registration.rs";

fn source(relative: &str, text: &str) -> SourceFile {
    SourceFile {
        absolute: PathBuf::from(relative),
        relative: relative.to_owned(),
        text: text.to_owned(),
    }
}

fn report(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn delegates_file_rules_to_every_runtime_safety_component() {
    let files = [
        source(BOUNDARY_PATH, "const THAW_VALUE: &str = \"0\\n\";"),
        source(PROCESS_PATH, "std::process::Command::new(\"sh\");"),
        source(PTY_PATH, "state.gate.try_lock_owned();"),
    ];
    let mut violations = Vec::new();
    for file in &files {
        crate::inspect_file(file, &mut violations);
    }
    let actual = report(&violations);
    assert!(actual.contains("must never expose or write cgroup thaw authority"));
    assert!(actual.contains("remote broker production code cannot create or escape"));
    assert!(actual.contains("typed FIFO mutation admission"));
}

#[test]
fn delegates_repository_rules_to_boundary_and_pty_components() {
    let files = [
        source(SOCKET_REGISTRATION_PATH, ""),
        source(PTY_SETTLEMENT_PATH, ""),
    ];
    let mut violations = Vec::new();
    crate::inspect_repository(&files, &mut violations);
    let actual = report(&violations);
    assert!(actual.contains("registered socket namespace publication contract is missing"));
    assert!(actual.contains("PTY master-closure capability definition"));
}
