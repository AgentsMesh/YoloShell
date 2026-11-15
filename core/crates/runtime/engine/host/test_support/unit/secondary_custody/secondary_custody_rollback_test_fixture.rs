pub(super) enum PreActivationReleaseIdentity {
    Exact,
    Forged,
}

#[derive(Clone, Copy)]
pub(super) enum PreActivationReleaseResponse {
    ReadAcknowledgement,
    DropWithoutReading,
    HoldWithoutReading,
    HoldThenObserveNoAcknowledgement,
}

const PRE_ACTIVATION_ACK_TIMEOUT: std::time::Duration =
    crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT
        .saturating_add(crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT);
const PRE_ACTIVATION_ACK_TIMEOUT_DETAIL: &str = "pre-activation Released acknowledgement timed out";
const PRE_ACTIVATION_NO_ACK_OBSERVATION_TIMEOUT: std::time::Duration =
    crate::host::PROCESS_EXIT_POLL_INTERVAL;
const UNEXPECTED_PRE_ACTIVATION_ACK_DETAIL: &str =
    "pre-activation rollback cancellation sent an acknowledgement";

pub(super) struct PreActivationLocalAliases {
    _aliases: crate::process::MacosSecondaryCustodyFixtureAliases,
    runtime_worker: Option<runtime_child::TestRuntimeChild>,
    runtime_child: Option<runtime_child::TestRuntimeChild>,
}

impl Drop for PreActivationLocalAliases {
    fn drop(&mut self) {
        if let Some(worker) = self.runtime_worker.as_mut() {
            worker.settle();
        }
        if let Some(child) = self.runtime_child.as_mut() {
            child.settle();
        }
    }
}

type PreActivationReleaseServer = (
    tokio::task::JoinHandle<()>,
    tokio::sync::oneshot::Receiver<()>,
    tokio::sync::oneshot::Sender<()>,
    Option<tokio::sync::oneshot::Sender<()>>,
);

impl MacosHandshakeFixture {
    pub(super) fn take_pre_activation_local_aliases(&mut self) -> PreActivationLocalAliases {
        PreActivationLocalAliases {
            _aliases: self.aliases.take().unwrap(),
            runtime_worker: self.runtime_worker.take(),
            runtime_child: self.runtime_child.take(),
        }
    }

    pub(super) fn serve_pre_activation_release(
        &mut self,
        identity: PreActivationReleaseIdentity,
        response: PreActivationReleaseResponse,
    ) -> PreActivationReleaseServer {
        let (listener, export, pending, custody_path, record) = self.take_parts();
        let (ready, reached_ready) = tokio::sync::oneshot::channel();
        let (send_release, release) = tokio::sync::oneshot::channel();
        let hold_open = matches!(
            response,
            PreActivationReleaseResponse::HoldWithoutReading
                | PreActivationReleaseResponse::HoldThenObserveNoAcknowledgement
        );
        let (finish, held) = if hold_open {
            let (finish, held) = tokio::sync::oneshot::channel();
            (Some(finish), Some(held))
        } else {
            (None, None)
        };
        let server = tokio::spawn(async move {
            let (mut stream, offer, request) = handshake_support::through_ready(
                &listener,
                export,
                pending,
                &custody_path,
                &record,
            )
            .await;
            ready.send(()).unwrap();
            release.await.unwrap();
            let released = match identity {
                PreActivationReleaseIdentity::Exact => offer,
                PreActivationReleaseIdentity::Forged => crate::host::test_custody_offer(
                    &request,
                    offer.descriptor_set,
                    offer.recovery_identity_sha256,
                ),
            };
            let control = crate::pty_custody::CustodyControl::release(&released).unwrap();
            let disposition =
                crate::pty_custody::CustodyWorkerReadyDisposition::release(control.clone());
            stream.write(&disposition).await.unwrap();
            if matches!(response, PreActivationReleaseResponse::ReadAcknowledgement) {
                let acknowledgement: crate::pty_custody::CustodyControlAck =
                    tokio::time::timeout(PRE_ACTIVATION_ACK_TIMEOUT, stream.read())
                        .await
                        .expect(PRE_ACTIVATION_ACK_TIMEOUT_DETAIL)
                        .unwrap();
                assert_eq!(
                    acknowledgement,
                    crate::pty_custody::CustodyControlAck::released(&control).unwrap()
                );
            }
            if let Some(held) = held {
                held.await.unwrap();
            }
            if matches!(
                response,
                PreActivationReleaseResponse::HoldThenObserveNoAcknowledgement
            ) {
                let acknowledgement = tokio::time::timeout(
                    PRE_ACTIVATION_NO_ACK_OBSERVATION_TIMEOUT,
                    stream.read::<crate::pty_custody::CustodyControlAck>(),
                )
                .await;
                assert!(
                    !matches!(acknowledgement, Ok(Ok(_))),
                    "{UNEXPECTED_PRE_ACTIVATION_ACK_DETAIL}"
                );
            }
        });
        (server, reached_ready, send_release, finish)
    }
}
