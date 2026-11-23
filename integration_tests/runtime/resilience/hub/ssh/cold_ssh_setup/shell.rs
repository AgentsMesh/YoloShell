use super::{constants::*, fixture::Fixture};
use crate::hub::relay_crash::process::RelayProcess;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::*;

pub struct Shell {
    pub target: RuntimeTarget,
    before: yolo_runtime::TerminalView,
    cleanup: Option<crate::support::RuntimeCleanup>,
}
impl Shell {
    pub async fn create(entry: &RelayProcess, fixture: &Fixture, peer: PeerIdentity) -> Self {
        super::evidence::wait_hub(entry, peer.hub()).await;
        let mut client = entry.client().await;
        let token = uuid::Uuid::new_v4();
        let request = CreateShellRequest::new(CreateShellFields {
            schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
            request_id: uuid::Uuid::new_v4(),
            creation_token: token,
            name: Some(SESSION_NAME.into()),
            cwd: fixture.home.to_str().unwrap().into(),
            size: TerminalSize::default(),
        })
        .unwrap();
        let host = fixture.direct_host();
        let status = host.status().await.unwrap();
        assert_eq!(status.host_pid, fixture.owners.identities().0.pid);
        let paths = crate::Paths::new(&fixture.home);
        let (mut cleanup, capture) =
            crate::support::RuntimeCleanup::capture_for_boot(&paths, &status.boot_id);
        capture.unwrap();
        let Reply::Created { creation } = client
            .call(peer.hub(), Command::Create { request }, deadline())
            .await
            .unwrap()
        else {
            panic!()
        };
        let before = host
            .get_session(creation.address.terminal_id.as_uuid())
            .await
            .unwrap();
        cleanup
            .bind_view_for_boot(&paths, &before, &status.boot_id)
            .unwrap();
        let target = RuntimeTarget {
            address: creation.address,
            runtime_epoch: RuntimeEpoch::from_uuid(before.runtime.as_ref().unwrap().runtime_epoch)
                .unwrap(),
        };
        let shell = Self {
            target,
            before,
            cleanup: Some(cleanup),
        };
        client
            .call(
                peer.hub(),
                Command::Finalize {
                    creation_token: token,
                    creation: creation.clone(),
                },
                deadline(),
            )
            .await
            .unwrap();
        let Reply::Terminal { terminal: session } = client
            .call(
                peer.hub(),
                Command::Get {
                    address: creation.address,
                },
                deadline(),
            )
            .await
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(session.fields().runtime.unwrap().target, target);
        shell.assert_survives(fixture).await;
        shell
    }
    pub async fn assert_survives(&self, fixture: &Fixture) {
        let current = fixture
            .direct_host()
            .get_session(self.target.address.terminal_id.as_uuid())
            .await
            .unwrap();
        let before = self.before.runtime.as_ref().unwrap();
        let runtime = current.runtime.as_ref().unwrap();
        assert_eq!(runtime.runtime_epoch, self.target.runtime_epoch.as_uuid());
        assert_eq!(runtime.runtime_epoch, before.runtime_epoch);
        assert_eq!(runtime.worker_id, before.worker_id);
        assert_eq!(runtime.worker_pid, before.worker_pid);
        assert_eq!(runtime.worker_start_time, before.worker_start_time);
        assert_eq!(runtime.child_pid, before.child_pid);
        assert_eq!(runtime.child_start_time, before.child_start_time);
        for identity in self.identities() {
            assert!(identity.matches());
        }
    }
    pub async fn stop(&mut self, entry: &RelayProcess) {
        entry
            .client()
            .await
            .call(
                self.target.address.hub_id,
                Command::Terminal {
                    command: TerminalCommand {
                        request_id: uuid::Uuid::new_v4(),
                        selection: TerminalSelection {
                            address: self.target.address,
                            expected_runtime: Some(self.target.runtime_epoch),
                        },
                        action: TerminalAction::Terminate { force: false },
                    },
                },
                deadline(),
            )
            .await
            .unwrap();
        tokio::time::timeout_at(deadline(), async {
            while self.identities().iter().any(|identity| {
                identity.observe() != yolo_process_test_support::ProcessIdentityObservation::Gone
            }) {
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .unwrap();
        self.cleanup.take().unwrap().finish().unwrap();
    }
    fn identities(&self) -> [crate::ProcessIdentity; 2] {
        let runtime = self.before.runtime.as_ref().unwrap();
        [
            crate::ProcessIdentity {
                pid: runtime.worker_pid,
                start_time: runtime.worker_start_time,
            },
            crate::ProcessIdentity {
                pid: runtime.child_pid,
                start_time: runtime.child_start_time,
            },
        ]
    }
}
impl Drop for Shell {
    fn drop(&mut self) {
        if let Some(mut cleanup) = self.cleanup.take() {
            cleanup.request();
            cleanup.force();
            if let Err(error) = cleanup.finish() {
                tracing::error!(%error, "cold setup shell cleanup failed");
            }
        }
    }
}
