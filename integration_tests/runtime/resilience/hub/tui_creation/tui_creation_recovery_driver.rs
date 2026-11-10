use super::{Driver, RunningHub, TUI_BINARY_ENV, deadline};
use crate::hub::tui_command::TERMINALS_COMMAND;
use crate::hub::tui_creation_recovery::constants::*;
use std::path::Path;
use yolo_process_test_support::{LiveProcessIdentity, ProcessIdentity as FixtureProcessIdentity};
use yolo_runtime::{
    agent::{FencedDescendants, ProcessSource, SystemProcessSource},
    process::{ProcessIdentity, ProcessIdentityObservation},
};

impl Driver {
    pub async fn crash_for_creation_recovery(mut self, hub: &RunningHub) -> tempfile::TempDir {
        let view = self
            .client
            .get_session(self.target.terminal_id)
            .await
            .unwrap();
        let runtime = view.runtime.as_ref().unwrap();
        assert_eq!(runtime.runtime_epoch, self.target.runtime_epoch);
        let root = ProcessIdentity {
            pid: runtime.child_pid,
            start_time: runtime.child_start_time,
        };
        let binary = crate::bazel_runfile(&std::env::var(TUI_BINARY_ENV).unwrap()).unwrap();
        let executable = binary.canonicalize().unwrap();
        let expected = vec![
            binary.to_str().unwrap().to_owned(),
            TERMINALS_COMMAND.into(),
            SOCKET_ARGUMENT.into(),
            hub.socket.to_str().unwrap().into(),
            HUB_ARGUMENT.into(),
            hub.hub.to_string(),
        ];
        let FencedDescendants::Stable(descendants) =
            SystemProcessSource.fenced_descendants(root).unwrap()
        else {
            panic!("owned outer PTY shell identity changed");
        };
        let candidates = descendants
            .iter()
            .filter(|process| {
                Path::new(&process.executable) == executable && process.argv == expected
            })
            .collect::<Vec<_>>();
        let [process] = candidates.as_slice() else {
            panic!("expected one exact TUI image and argv beneath the owned outer PTY shell");
        };
        let identity = ProcessIdentity {
            pid: process.pid,
            start_time: process.start_time,
        };
        // Crash only this exact frontend. A discovered lineage is not a Linux
        // process-tree termination authority; retain the pidfd/kernel identity.
        let retained = LiveProcessIdentity::capture_for_test(FixtureProcessIdentity {
            pid: identity.pid,
            start_time: identity.start_time,
        })
        .unwrap()
        .expect("exact TUI process exited before the crash checkpoint");
        assert!(root.matches() && identity.matches());
        assert!(retained.terminate_retained_fixture().unwrap());
        let complete = self.complete.clone();
        self.wait(|text| text.contains(&complete)).await;
        tokio::time::timeout_at(deadline(), async {
            while identity.observe() != ProcessIdentityObservation::Gone {
                tokio::time::sleep(yolo_terminal_query_fixture::constants::POLL).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(identity.observe(), ProcessIdentityObservation::Gone);
        // Keep the production TUI's profile, including after its forced exit.
        // The collector remains responsible for validating the retained evidence.
        self.output.close().await;
        // Only this test's outer PTY is retired. The origin runtime is owned by
        // another Host and cannot belong to the captured frontend lineage.
        self.client
            .terminate(self.target.terminal_id, false)
            .await
            .unwrap();
        self.workspace
    }
}
