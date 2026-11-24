use super::{constants::*, deadline, fixture::Fixture};
use yolo_hub_client::EndpointClient;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::*;

impl Fixture {
    pub fn create_request(&self) -> CreateShellRequest {
        CreateShellRequest::new(CreateShellFields {
            schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
            request_id: uuid::Uuid::new_v4(),
            creation_token: uuid::Uuid::new_v4(),
            name: Some(SESSION_NAME.into()),
            cwd: self.host.home.path().display().to_string(),
            size: TerminalSize::default(),
        })
        .unwrap()
    }

    pub async fn create(&self) -> (TerminalSummary, yolo_runtime::TerminalView) {
        let request = self.create_request();
        let mut client = self.process.client().await;
        let Reply::Created { creation } = client
            .call(
                self.hub,
                Command::Create {
                    request: request.clone(),
                },
                deadline(),
            )
            .await
            .unwrap()
        else {
            panic!("missing shell creation result")
        };
        let view = self
            .host
            .client()
            .get_session(creation.address.terminal_id.as_uuid())
            .await
            .unwrap();
        self.host.track_view(&view);
        client
            .call(
                self.hub,
                Command::Finalize {
                    creation_token: request.fields().creation_token,
                    creation,
                },
                deadline(),
            )
            .await
            .unwrap();
        let Reply::Terminal { terminal: session } = client
            .call(
                self.hub,
                Command::Get {
                    address: creation.address,
                },
                deadline(),
            )
            .await
            .unwrap()
        else {
            panic!("missing finalized shell")
        };
        (session, view)
    }

    pub fn lease_command(&self, target: RuntimeTarget, mode: LeaseMode) -> Command {
        Command::Lease {
            request: LeaseRequest {
                request_id: uuid::Uuid::new_v4(),
                target,
                principal: WriterPrincipal {
                    client_id: self.identity.client_id,
                    controller_instance_id: uuid::Uuid::new_v4(),
                },
                mode,
            },
        }
    }

    pub async fn acquire(&self, client: &mut EndpointClient, target: RuntimeTarget) -> LeaseGrant {
        let Reply::LeaseObservation { lease } = self
            .process
            .client()
            .await
            .call(self.hub, Command::ObserveLease { target }, deadline())
            .await
            .unwrap()
        else {
            panic!("missing writer observation")
        };
        let mode = lease.map_or(LeaseMode::Acquire {}, |lease| LeaseMode::TakeControl {
            observed_generation: lease.fields().generation,
        });
        let Reply::Lease { grant } = client
            .call(self.hub, self.lease_command(target, mode), deadline())
            .await
            .unwrap()
        else {
            panic!("missing writer grant")
        };
        grant
    }

    pub async fn custody(&self, original: &yolo_runtime::TerminalView) {
        let current = self
            .host
            .client()
            .get_session(original.manifest.terminal_id)
            .await
            .unwrap();
        assert!(!current.manifest.archived);
        let before = original.runtime.as_ref().unwrap();
        let after = current.runtime.as_ref().unwrap();
        assert_eq!(after.runtime_epoch, before.runtime_epoch);
        assert_eq!(after.worker_id, before.worker_id);
        assert_eq!(after.worker_pid, before.worker_pid);
        assert_eq!(after.worker_start_time, before.worker_start_time);
        assert_eq!(after.child_pid, before.child_pid);
        assert_eq!(after.child_start_time, before.child_start_time);
        for (pid, start_time) in [
            (after.worker_pid, after.worker_start_time),
            (after.child_pid, after.child_start_time),
        ] {
            assert!(crate::ProcessIdentity { pid, start_time }.matches());
        }
    }

    pub async fn no_writer(&self, target: RuntimeTarget) {
        tokio::time::timeout_at(deadline(), async {
            loop {
                let Reply::LeaseObservation { lease } = self
                    .process
                    .client()
                    .await
                    .call(self.hub, Command::ObserveLease { target }, deadline())
                    .await
                    .unwrap()
                else {
                    panic!("missing writer observation")
                };
                if lease.is_none() {
                    return;
                }
                tokio::time::sleep(yolo_terminal_query_fixture::constants::POLL).await;
            }
        })
        .await
        .unwrap();
    }
}

pub fn marker() -> (Vec<u8>, String) {
    let nonce = uuid::Uuid::new_v4();
    (
        format!("printf '%s%s%s\\n' '{MARKER_PREFIX}' '{MARKER_SUFFIX}' '{nonce}'\r").into_bytes(),
        format!("{MARKER_PREFIX}{MARKER_SUFFIX}{nonce}"),
    )
}
