use super::cleanup_tree::RuntimeProcessTrees;
use super::runtime_record_test_support::{record_for_identity, session_view};
use super::*;
use std::os::unix::fs::PermissionsExt;

const DESCENDANT_SCRIPT_CONTENT: &str =
    "#!/bin/sh\ntrap '' HUP TERM\necho $$ > \"$1\"\nwhile :; do sleep 1; done\n";
const DESCENDANT_SCRIPT_NAME: &str = "cleanup-descendant.sh";
const ROOT_SCRIPT_CONTENT: &str = "#!/bin/sh\ntrap '' HUP TERM\n\"$1\" \"$2\" &\nwait\n";
const ROOT_SCRIPT_NAME: &str = "cleanup-successor.sh";
const DESCENDANT_PID_FILE: &str = "cleanup-descendant.pid";
const SCRIPT_MODE: u32 = 0o700;
const PROCESS_DISCOVERY_TIMEOUT: Duration = PROCESS_EXIT_TIMEOUT;
const DESCENDANT_DISCOVERY_FAILURE_DETAIL: &str = "cleanup descendant identity was not published";

#[test]
fn immediate_successor_binding_reaps_descendants_after_root_and_record_loss() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let fixture_directory = tempfile::tempdir().unwrap();
    let root_script = write_script(
        fixture_directory.path(),
        ROOT_SCRIPT_NAME,
        ROOT_SCRIPT_CONTENT,
    );
    let descendant_script = write_script(
        fixture_directory.path(),
        DESCENDANT_SCRIPT_NAME,
        DESCENDANT_SCRIPT_CONTENT,
    );
    let pid_file = fixture_directory.path().join(DESCENDANT_PID_FILE);
    let mut root = ChildGuard::new(
        Command::new(&root_script)
            .args([descendant_script.as_os_str(), pid_file.as_os_str()])
            .spawn()
            .unwrap(),
    );
    let root_identity = ProcessIdentity::capture(root.id()).unwrap();
    let descendant = wait_for_identity(&pid_file);
    let _descendant_guard = IdentityGuard(descendant);
    let record = record_for_identity(&paths, Uuid::new_v4(), root_identity);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();
    let view = session_view(&record);
    let (mut cleanup, capture) =
        RuntimeCleanup::with_trees(&paths, &[], RuntimeProcessTrees::default());
    assert!(capture.is_err());
    cleanup.bind_view(&paths, &view).unwrap();

    assert!(root_identity.signal(libc::SIGKILL).unwrap());
    root.wait().unwrap();
    assert!(descendant.matches());
    std::fs::remove_file(paths.runtime_record(record.terminal_id)).unwrap();

    cleanup.force();
    assert!(cleanup.finish().is_err());
    assert_eq!(
        descendant.observe(),
        yolo_process_test_support::ProcessIdentityObservation::Gone
    );
}

struct IdentityGuard(ProcessIdentity);

impl Drop for IdentityGuard {
    fn drop(&mut self) {
        let _ = self.0.signal(libc::SIGKILL);
    }
}

fn write_script(root: &Path, name: &str, contents: &str) -> PathBuf {
    let path = root.join(name);
    std::fs::write(&path, contents).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(SCRIPT_MODE)).unwrap();
    path
}

fn wait_for_identity(path: &Path) -> ProcessIdentity {
    let deadline = Instant::now() + PROCESS_DISCOVERY_TIMEOUT;
    loop {
        match read_identity(path) {
            Ok(identity) => return identity,
            Err(diagnostic) => assert!(
                Instant::now() < deadline,
                "{DESCENDANT_DISCOVERY_FAILURE_DETAIL}: {diagnostic}"
            ),
        }
        std::thread::sleep(PROCESS_POLL_INTERVAL);
    }
}

fn read_identity(path: &Path) -> Result<ProcessIdentity, String> {
    let contents = std::fs::read_to_string(path).map_err(|error| format!("read={error}"))?;
    let pid = contents
        .trim()
        .parse::<u32>()
        .map_err(|error| format!("pid={contents:?}, parse={error}"))?;
    yolo_process_test_support::process_start_time(pid)
        .map(|start_time| ProcessIdentity { pid, start_time })
        .map_err(|error| format!("pid={pid}, identity={error:#}"))
}
