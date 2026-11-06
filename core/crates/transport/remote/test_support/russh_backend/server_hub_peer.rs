macro_rules! hub_peer_handler {
    () => {
        async fn channel_open_direct_streamlocal(
            &mut self,
            channel: Channel<Msg>,
            socket_path: &str,
            reply: russh::server::ChannelOpenHandle,
            _: &mut Session,
        ) -> Result<(), Self::Error> {
            let allowed = self.filesystem.hub_peer_target();
            if !allowed
                .as_ref()
                .is_some_and(|target| target.as_path() == std::path::Path::new(socket_path))
            {
                reply
                    .reject(ChannelOpenFailure::AdministrativelyProhibited)
                    .await;
                return Ok(());
            }
            self.hub_peers.insert(channel.id());
            let channel_id = channel.id();
            let filesystem = self.filesystem.clone();
            let delayed = filesystem.take_channel_open_failure();
            tokio::spawn(async move {
                if delayed && !filesystem.permit_channel_open().await {
                    return;
                }
                let mut socket = tokio::net::UnixStream::connect(allowed.unwrap())
                    .await
                    .unwrap();
                reply.accept().await;
                filesystem.record_hub_peer_accepted(channel_id);
                filesystem.record_channel_open_accepted();
                let mut stream = channel.into_stream();
                let _ = tokio::io::copy_bidirectional(&mut stream, &mut socket).await;
                drop((stream, socket));
                filesystem.record_hub_peer_forwarder_finished(channel_id);
            });
            Ok(())
        }
    };
}

pub(super) use hub_peer_handler;
