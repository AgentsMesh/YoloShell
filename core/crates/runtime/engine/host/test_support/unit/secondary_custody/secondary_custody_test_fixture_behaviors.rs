use super::handshake_support;
use crate::pty_custody::{CustodyControl, CustodyControlAck};

pub(super) enum ActivationFixture {
    Missing,
    Forged,
}

pub(super) enum ReleaseIdentity {
    Exact,
    Forged,
}

pub(super) type FixtureParts = (
    UnixListener,
    MacosSecondaryCustodyExport,
    CustodyDurableRecord,
    std::path::PathBuf,
    RuntimeRecord,
);

impl MacosHandshakeFixture {
    pub(super) fn serve(&mut self) -> tokio::task::JoinHandle<()> {
        let (listener, export, pending, custody_path, record) = self.take_parts();
        tokio::spawn(async move {
            let (stream, _, _) = handshake_support::complete_handshake(
                &listener,
                export,
                pending,
                &custody_path,
                &record,
            )
            .await;
            let _authenticated = handshake_support::authenticate(&listener, &record).await;
            drop(stream);
        })
    }

    pub(super) fn serve_release(
        &mut self,
        identity: ReleaseIdentity,
        response: super::ReleaseResponse,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (listener, export, pending, custody_path, record) = self.take_parts();
        let (release, released) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, offer, request) = handshake_support::complete_handshake(
                &listener,
                export,
                pending,
                &custody_path,
                &record,
            )
            .await;
            released.await.unwrap();
            let released = match identity {
                ReleaseIdentity::Exact => offer,
                ReleaseIdentity::Forged => crate::host::test_custody_offer(
                    &request,
                    offer.descriptor_set,
                    offer.recovery_identity_sha256,
                ),
            };
            let control = CustodyControl::release(&released).unwrap();
            stream.write(&control).await.unwrap();
            if matches!(response, super::ReleaseResponse::ReadAcknowledgement) {
                let acknowledgement: CustodyControlAck = stream.read().await.unwrap();
                assert_eq!(
                    acknowledgement,
                    CustodyControlAck::released(&control).unwrap()
                );
            }
        });
        (server, release)
    }

    pub(super) fn serve_holding(
        &mut self,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (listener, export, pending, custody_path, record) = self.take_parts();
        let (release, released) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (stream, _, _) = handshake_support::complete_handshake(
                &listener,
                export,
                pending,
                &custody_path,
                &record,
            )
            .await;
            released.await.unwrap();
            drop(stream);
        });
        (server, release)
    }

    pub(super) fn serve_truncated_control_holding(
        &mut self,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (listener, export, pending, custody_path, record) = self.take_parts();
        let (release, released) = tokio::sync::oneshot::channel();
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
            let control = CustodyControl::release(&offer).unwrap();
            crate::pty_custody::test_support::write_truncated_frame(&mut stream, &control)
                .await
                .unwrap();
        });
        (server, release)
    }

    pub(super) fn serve_activation(
        &mut self,
        behavior: ActivationFixture,
    ) -> tokio::task::JoinHandle<()> {
        let (listener, export, pending, custody_path, record) = self.take_parts();
        tokio::spawn(async move {
            match behavior {
                ActivationFixture::Missing => {
                    handshake_support::close_without_activation(
                        &listener,
                        export,
                        pending,
                        &custody_path,
                        &record,
                    )
                    .await
                }
                ActivationFixture::Forged => {
                    handshake_support::send_forged_activation(
                        &listener,
                        export,
                        pending,
                        &custody_path,
                        &record,
                    )
                    .await
                }
            }
        })
    }

    pub(super) fn serve_after_prepared_holding(
        &mut self,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::sync::oneshot::Receiver<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (listener, export, _pending, _custody_path, record) = self.take_parts();
        let (prepared, reached_prepared) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            handshake_support::hold_after_prepared(&listener, export, &record, prepared, released)
                .await;
        });
        (server, reached_prepared, release)
    }

    pub(super) fn take_parts(&mut self) -> FixtureParts {
        (
            self.listener.take().unwrap(),
            self.export.take().unwrap(),
            self.pending.take().unwrap(),
            self.custody_path.clone(),
            self.record.clone(),
        )
    }
}
