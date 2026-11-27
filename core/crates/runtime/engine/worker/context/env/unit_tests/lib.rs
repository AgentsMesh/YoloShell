use crate::apply;
use portable_pty::CommandBuilder;
use std::ffi::OsStr;
use uuid::Uuid;

#[test]
fn caller_context_injects_hub_workspace_and_run_identity() {
    let mut command = CommandBuilder::new("sh");
    let hub = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    let run = Uuid::new_v4();
    apply(&mut command, hub, Some(workspace), Some(run));
    let hub_value = hub.to_string();
    let workspace_value = workspace.to_string();
    let run_value = run.to_string();
    assert_eq!(
        command.get_env("YOLO_HUB_ID"),
        Some(OsStr::new(&hub_value))
    );
    assert_eq!(
        command.get_env("YOLO_WORKSPACE_ID"),
        Some(OsStr::new(&workspace_value))
    );
    assert_eq!(
        command.get_env("YOLO_RUN_ID"),
        Some(OsStr::new(&run_value))
    );
}

#[test]
fn a_terminal_without_a_run_or_workspace_gets_no_such_variables() {
    let mut command = CommandBuilder::new("sh");
    apply(&mut command, Uuid::new_v4(), None, None);
    assert_eq!(command.get_env("YOLO_WORKSPACE_ID"), None);
    assert_eq!(command.get_env("YOLO_RUN_ID"), None);
}
