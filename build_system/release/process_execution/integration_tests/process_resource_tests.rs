use super::child_lifecycle::{SLEEP_ARGUMENT, SLEEP_EXECUTABLE, assert_reaped};
use super::resources::Resources;
use crate::constants::NULL_DEVICE;
use crate::posix_cleanup::terminate_process;
use crate::posix_spawn::{SpawnedProcess, spawn};
use crate::posix_supervisor::Streams;
use crate::process_resources::ProcessResources;
use std::fs::{self, File};
use std::process::Command;
use std::time::Duration;

const DROP_REAP_GRACE: Duration = Duration::from_millis(25);
const COMMAND_TIMEOUT: Duration = Duration::from_millis(1);
const MISSING_EXECUTABLE: &str = "/nonexistent-yolo-custody-test-command";
const NONZERO_EXECUTABLE: &str = "/usr/bin/false";

#[test]
fn unresolved_live_child_preserves_working_home_temp_and_tool_roots() {
    let owners = Resources::new();
    let command = owners.command(SLEEP_EXECUTABLE, &[SLEEP_ARGUMENT]);
    let paths = owners.paths();
    let mut resources = ProcessResources::acquire(&command).unwrap();
    let mut direct = Command::new(SLEEP_EXECUTABLE)
        .arg(SLEEP_ARGUMENT)
        .current_dir(owners.workspace.path())
        .spawn()
        .unwrap();
    resources.mark_spawned();
    let output = File::open(NULL_DEVICE).unwrap();
    let mut child = SpawnedProcess::new(
        direct.id().try_into().unwrap(),
        output.try_clone().unwrap(),
        output,
        None,
    );
    child.resources = resources;
    child.configure_drop_fault_for_test(DROP_REAP_GRACE);
    drop(child);
    drop(owners);
    let running = direct.try_wait().unwrap().is_none();
    let preserved = paths.iter().all(|path| path.is_dir());
    if running {
        direct.kill().unwrap();
        direct.wait().unwrap();
    }
    assert!(
        running,
        "fixture must retain a real live child through owner drop"
    );
    assert!(preserved, "unsettled child resources were deleted");
    for path in paths {
        fs::remove_dir_all(path).unwrap();
    }
}

#[test]
fn real_spawn_holds_owner_before_explicit_child_settlement() {
    let owners = Resources::new();
    let command = owners.command(SLEEP_EXECUTABLE, &[SLEEP_ARGUMENT]);
    let paths = owners.paths();
    let mut child = spawn(&command).unwrap();
    let pid = child.pid;
    drop(owners);
    let preserved = paths.iter().all(|path| path.is_dir());
    terminate_process(&mut child, &mut Streams::default()).unwrap();
    assert_reaped(pid);
    assert!(preserved, "spawned command must hold its directory owners");
    for path in paths {
        fs::remove_dir_all(path).unwrap();
    }
}

#[test]
fn known_settled_timeout_and_nonzero_and_never_started_failures_allow_cleanup() {
    let owners = Resources::new();
    let paths = owners.paths();
    assert!(owners.command(MISSING_EXECUTABLE, &[]).execute().is_err());
    drop(owners);
    assert!(paths.iter().all(|path| !path.exists()));

    let owners = Resources::new();
    let paths = owners.paths();
    assert!(
        !owners
            .command(NONZERO_EXECUTABLE, &[])
            .execute()
            .unwrap()
            .status
            .success()
    );
    drop(owners);
    assert!(paths.iter().all(|path| !path.exists()));

    let owners = Resources::new();
    let paths = owners.paths();
    let result = owners
        .command(SLEEP_EXECUTABLE, &[SLEEP_ARGUMENT])
        .with_timeout(COMMAND_TIMEOUT)
        .execute();
    assert!(result.is_err());
    drop(owners);
    assert!(paths.iter().all(|path| !path.exists()));
}

#[test]
fn acquisition_failure_before_spawn_does_not_poison_other_resource_owners() {
    let mut owners = Resources::new();
    let command = owners.command(SLEEP_EXECUTABLE, &[SLEEP_ARGUMENT]);
    owners.tool.cleanup().unwrap();
    let paths = owners.paths();
    assert!(command.execute().is_err());
    drop(owners);
    assert!(paths.iter().all(|path| !path.exists()));
}

#[test]
fn positive_fallback_settlement_releases_owned_directories() {
    let owners = Resources::new();
    let paths = owners.paths();
    let child = spawn(&owners.command(SLEEP_EXECUTABLE, &[SLEEP_ARGUMENT])).unwrap();
    let pid = child.pid;
    drop(child);
    assert_reaped(pid);
    drop(owners);
    assert!(paths.iter().all(|path| !path.exists()));
}
