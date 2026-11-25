use super::{constants::*, fixture::Installation};
use crate::hub::{
    federation_support::{POLL, deadline},
    support::MAX_OBSERVED_BYTES,
};
use crate::{Harness, ProcessIdentity, runtime_ref_from_view};
use std::{fs, os::unix::fs::OpenOptionsExt, path::PathBuf};
use yolo_runtime::{
    model::{TerminalRuntimeStatus, TerminalView},
    protocol::{event, from_value},
};

pub(super) struct ShellProbe {
    original: TerminalView,
    worker: ProcessIdentity,
    shell: ProcessIdentity,
    marker_path: PathBuf,
    expected_interrupt: Vec<u8>,
}

impl ShellProbe {
    pub async fn prepare(
        host: &Harness,
        original: &TerminalView,
        installation: &Installation,
    ) -> Self {
        let runtime = original.runtime.as_ref().unwrap();
        let marker_path = installation
            .profile_path
            .parent()
            .unwrap()
            .join(SHELL_INTERRUPT_FILE);
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(PRIVATE_FILE_MODE)
            .open(&marker_path)
            .unwrap();
        let marker = uuid::Uuid::new_v4().to_string();
        let append = format!(
            "printf '%s\\n' '{}' >> '{}'",
            marker,
            marker_path.to_string_lossy().replace('\'', QUOTE_ESCAPE)
        );
        let setup = format!("trap '{}' INT", append.replace('\'', QUOTE_ESCAPE));
        let probe = Self {
            original: original.clone(),
            worker: ProcessIdentity {
                pid: runtime.worker_pid,
                start_time: runtime.worker_start_time,
            },
            shell: ProcessIdentity {
                pid: runtime.child_pid,
                start_time: runtime.child_start_time,
            },
            marker_path,
            expected_interrupt: format!("{marker}\n").into_bytes(),
        };
        probe.alive();
        probe.exchange(host, Some(&setup)).await;
        assert!(fs::read(&probe.marker_path).unwrap().is_empty());
        probe
    }

    pub async fn interrupt_delivered_once(&self) {
        tokio::time::timeout_at(deadline(), async {
            loop {
                let actual = fs::read(&self.marker_path).unwrap();
                if actual == self.expected_interrupt {
                    break;
                }
                assert!(
                    self.expected_interrupt.starts_with(&actual),
                    "unexpected or repeated target shell interrupt"
                );
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .unwrap();
    }

    pub async fn after_tui_exit(&self, host: &Harness) {
        self.alive();
        let observed = host
            .client()
            .get_session(self.original.manifest.terminal_id)
            .await
            .unwrap();
        assert!(!observed.manifest.archived);
        let before = self.original.runtime.as_ref().unwrap();
        let after = observed.runtime.as_ref().unwrap();
        assert_eq!(
            (
                after.runtime_epoch,
                after.worker_id,
                after.worker_pid,
                after.worker_start_time,
                after.child_pid,
                after.child_start_time
            ),
            (
                before.runtime_epoch,
                before.worker_id,
                before.worker_pid,
                before.worker_start_time,
                before.child_pid,
                before.child_start_time
            )
        );
        assert!(matches!(
            after.status,
            TerminalRuntimeStatus::Running | TerminalRuntimeStatus::OutputBlocked
        ));
        self.exchange(host, None).await;
        self.alive();
        assert_eq!(
            fs::read(&self.marker_path).unwrap(),
            self.expected_interrupt,
            "external TUI shutdown must not signal its target shell"
        );
    }

    pub async fn restore(&self, host: &Harness) {
        self.exchange(host, Some("trap - INT")).await;
    }

    fn alive(&self) {
        assert!(self.worker.matches() && self.shell.matches());
    }

    async fn exchange(&self, host: &Harness, setup: Option<&str>) {
        let target = runtime_ref_from_view(&self.original);
        let client = host.client();
        tokio::time::timeout_at(deadline(), async {
            while client.get_writer_lease(target).await.unwrap().is_some() {
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .unwrap();
        let grant = client
            .acquire_writer(
                target,
                SHELL_PROBE_WRITER,
                Some(yolo_runtime::lease::constants::MAX_TTL_MS),
            )
            .await
            .unwrap();
        let mut output = client
            .attach(
                self.original.manifest.terminal_id,
                yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE,
            )
            .await
            .unwrap();
        let nonce = uuid::Uuid::new_v4();
        let marker = format!("{SHELL_MARKER_PREFIX}{nonce}");
        let command = format!(
            "{}printf '%s%s\\n' '{SHELL_MARKER_PREFIX}' '{nonce}'\r",
            setup.map(|setup| format!("{setup}; ")).unwrap_or_default()
        );
        client
            .input(
                target,
                grant.generation,
                &grant.writer_capability,
                command.into_bytes(),
            )
            .await
            .unwrap();
        let mut observed = Vec::new();
        tokio::time::timeout_at(deadline(), async {
            loop {
                let message = output.next_event().await.unwrap();
                assert_ne!(message.event, event::SESSION_EXITED);
                if message.event == event::TERMINAL_OUTPUT {
                    let chunk: yolo_runtime::model::OutputChunk =
                        from_value(message.payload).unwrap();
                    observed.extend_from_slice(&chunk.data);
                    assert!(observed.len() <= MAX_OBSERVED_BYTES);
                    if String::from_utf8_lossy(&observed).contains(&marker) {
                        break;
                    }
                }
            }
        })
        .await
        .unwrap();
        client.release_writer(&grant, target).await.unwrap();
        output.detach().await.unwrap();
    }
}
