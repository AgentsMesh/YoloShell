macro_rules! hub_peer_handler {
    () => {
        async fn channel_open_direct_tcpip(
            &mut self,
            channel: Channel<Msg>,
            host: &str,
            port: u32,
            _: &str,
            _: u32,
            reply: russh::server::ChannelOpenHandle,
            _: &mut Session,
        ) -> Result<(), Self::Error> {
            let allowed = self.filesystem.hub_peer_target();
            if !allowed.is_some_and(|target| {
                host == target.ip().to_string() && port == u32::from(target.port())
            }) {
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
                let mut socket = tokio::net::TcpStream::connect(allowed.unwrap())
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
