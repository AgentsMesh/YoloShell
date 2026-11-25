type HeldActiveReleaseServer = (
    tokio::task::JoinHandle<()>,
    tokio::sync::oneshot::Sender<()>,
    tokio::sync::oneshot::Receiver<()>,
    tokio::sync::oneshot::Sender<()>,
);

type HeldSupersedeServer = (
    tokio::task::JoinHandle<()>,
    tokio::sync::oneshot::Receiver<()>,
    tokio::sync::oneshot::Sender<()>,
);

#[derive(Clone, Copy)]
enum HeldSupersedeResponse {
    Acknowledge,
    EndOfStream,
}

const EXPECTED_ACTIVE_RELEASE_ACK_DETAIL: &str =
    "settled active PTY custody Release did not receive its acknowledgement";
const EXPECTED_SUPERSEDE_CONTROL_DETAIL: &str =
    "held predecessor expected an exact Supersede control";

impl MacosHandshakeFixture {
    pub(super) fn serve_held_active_release(&mut self) -> HeldActiveReleaseServer {
        let (listener, export, pending, custody_path, record) = self.take_parts();
        let (release, released) = tokio::sync::oneshot::channel();
        let (sent, release_sent) = tokio::sync::oneshot::channel();
        let (finish, held) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, offer, _) = handshake_support::complete_handshake(
                &listener,
                export,
                pending,
                &custody_path,
                &record,
            )
            .await;
            released.await.unwrap();
            let control = crate::pty_custody::CustodyControl::release(&offer).unwrap();
            stream.write(&control).await.unwrap();
            sent.send(()).unwrap();
            held.await.unwrap();
            let acknowledgement: crate::pty_custody::CustodyControlAck = tokio::time::timeout(
                crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT,
                stream.read::<crate::pty_custody::CustodyControlAck>(),
            )
            .await
            .expect(EXPECTED_ACTIVE_RELEASE_ACK_DETAIL)
            .expect(EXPECTED_ACTIVE_RELEASE_ACK_DETAIL);
            assert_eq!(
                acknowledgement,
                crate::pty_custody::CustodyControlAck::released(&control).unwrap()
            );
        });
        (server, release, release_sent, finish)
    }

    pub(super) fn serve_held_supersede(&mut self) -> HeldSupersedeServer {
        self.serve_held_supersede_with(HeldSupersedeResponse::Acknowledge)
    }

    pub(super) fn serve_held_supersede_eof(&mut self) -> HeldSupersedeServer {
        self.serve_held_supersede_with(HeldSupersedeResponse::EndOfStream)
    }

    fn serve_held_supersede_with(
        &mut self,
        response: HeldSupersedeResponse,
    ) -> HeldSupersedeServer {
        let (listener, export, pending, custody_path, record) = self.take_parts();
        let (received, supersede_received) = tokio::sync::oneshot::channel();
        let (finish, held) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _, _) = handshake_support::complete_handshake(
                &listener,
                export,
                pending,
                &custody_path,
                &record,
            )
            .await;
            let control: crate::pty_custody::CustodyControl = stream.read().await.unwrap();
            let acknowledgement = crate::pty_custody::CustodyControlAck::superseded(&control)
                .expect(EXPECTED_SUPERSEDE_CONTROL_DETAIL);
            received.send(()).unwrap();
            held.await.unwrap();
            if matches!(response, HeldSupersedeResponse::Acknowledge) {
                stream.write(&acknowledgement).await.unwrap();
            }
        });
        (server, supersede_received, finish)
    }
}
