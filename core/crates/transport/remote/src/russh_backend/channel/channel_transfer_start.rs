impl TransferredChannelCustody {
    pub(super) fn start_bundle(
        session: Arc<Handle<HostKeyHandler>>,
        channel: SessionChannel,
        approved: ApprovalBuffer,
        reservation: RemoteCleanupReservation,
        cleanup_timeout: Duration,
    ) -> (DuplexStream, Self) {
        let (stream, close, task) = super::hub_peer_driver::start_buffered(channel, approved);
        (
            stream,
            Self::with_driver(session, close, task, reservation, cleanup_timeout),
        )
    }
    pub(super) fn start(
        session: Arc<Handle<HostKeyHandler>>,
        channel: SessionChannel,
        approved: ApprovalBuffer,
        reservation: RemoteCleanupReservation,
        cleanup_timeout: Duration,
    ) -> (DuplexStream, Self) {
        let (stream, driver) = sftp_channel_driver::start(channel, approved);
        (
            stream,
            Self::with_driver(
                session,
                driver.close,
                driver.task,
                reservation,
                cleanup_timeout,
            ),
        )
    }

    pub(super) fn start_hub_peer(
        session: Arc<Handle<HostKeyHandler>>,
        channel: SessionChannel,
        reservation: RemoteCleanupReservation,
        cleanup_timeout: Duration,
    ) -> (DuplexStream, Self) {
        let (stream, close, task) = super::hub_peer_driver::start(channel);
        (
            stream,
            Self::with_driver(session, close, task, reservation, cleanup_timeout),
        )
    }

    fn with_driver(
        session: Arc<Handle<HostKeyHandler>>,
        close: oneshot::Sender<()>,
        task: DriverTask,
        reservation: RemoteCleanupReservation,
        cleanup_timeout: Duration,
    ) -> Self {
        Self {
            session: Some(session),
            close: Some(close),
            task: Some(task),
            reservation: Some(reservation),
            cleanup_timeout,
            disconnect_required: false,
        }
    }
}
