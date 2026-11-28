use crate::identity::managed_launch::{LEGACY_SETTING, NO_DAEMON, resume};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};
use yolo_agent_contract::ResumePlan;
use yolo_codex::codex::constants::{CONFIG_ARGUMENT, EXECUTABLE, RESUME_ARGUMENT};

use super::constants;

#[test]
fn terminal_path_selects_modern_and_legacy_args_without_replaying_a_failed_launch() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join(EXECUTABLE);
    fs::write(&executable, constants::FAKE).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let cwd = "/tmp/QA's build";
    let session = "saved-session";
    let plan = resume(ResumePlan {
        executable: EXECUTABLE.into(),
        args: vec![
            RESUME_ARGUMENT.into(),
            "-C".into(),
            cwd.into(),
            session.into(),
        ],
        environment: Default::default(),
    });
    for (probe, prefix) in [
        ("0", format!("<{NO_DAEMON}>\n<--disable>\n<network_proxy>\n<--disable>\n<system_proxy_fallback>\n")),
        ("2", format!("<{CONFIG_ARGUMENT}>\n<{LEGACY_SETTING}>\n")),
    ] {
        let mut start = Command::new("/bin/sh");
        start.args(["-c", crate::runtime().launch_command().unwrap()]);
        let mut recover = Command::new(&plan.executable);
        recover.args(&plan.args);
        for (mut command, suffix) in [
            (start, String::new()),
            (
                recover,
                format!("<{RESUME_ARGUMENT}>\n<-C>\n<{cwd}>\n<{session}>\n"),
            ),
        ] {
            let result = command
                .env("PATH", root.path())
                .env("PROBE_STATUS", probe)
                .output()
                .unwrap();
            assert_eq!(result.status.code(), Some(17));
            assert_eq!(
                String::from_utf8(result.stdout).unwrap(),
                format!("{prefix}{suffix}")
            );
        }
    }
}
