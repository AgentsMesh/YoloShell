use super::*;

impl Fixture {
    pub async fn setup(
        &mut self,
        entry: &RelayProcess,
        request: &RemoteSetupRequest,
        action: SetupAction,
    ) -> RemoteSetupStatus {
        let end = deadline();
        let started = tokio::time::Instant::now();
        let mut action = action;
        loop {
            self.owners.observe().await;
            if !self.broker_captured {
                self.broker_captured = self.broker.capture_if_ready().await;
            }
            let result = tokio::time::timeout_at(end, async {
                let mut client = entry.client().await;
                client
                    .call(
                        self.entry_peer.hub(),
                        Command::RemoteSetup {
                            call: RemoteSetupCall {
                                action,
                                request: request.clone(),
                                credentials: None,
                            },
                        },
                        end,
                    )
                    .await
            })
            .await;
            let Reply::RemoteSetup { status } = result.unwrap().unwrap() else {
                panic!()
            };
            status.validate_for(request).unwrap();
            match status.outcome {
                RemoteSetupOutcome::Running {} => {
                    action = SetupAction::Poll;
                    tokio::time::sleep(POLL).await;
                }
                RemoteSetupOutcome::Linked { .. } => {
                    self.owners.observe().await;
                    assert!(self.owners.complete());
                    return status;
                }
                _ => {
                    let history = entry
                        .client()
                        .await
                        .inspect_remote_setup(request.request.fields().request_id, deadline())
                        .await;
                    let broker = self.broker.client().status().await;
                    panic!(
                        "{SETUP_FAILED}: {status:?}; elapsed={:?}; history={history:?}; broker={broker:?}; entry_stderr={}",
                        started.elapsed(),
                        entry.diagnostic()
                    );
                }
            }
            assert!(tokio::time::Instant::now() < end, "{SETUP_FAILED}");
        }
    }
}
