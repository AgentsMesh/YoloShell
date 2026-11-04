macro_rules! channel_open_handler {
    () => {
        async fn channel_open_session(
            &mut self,
            channel: Channel<Msg>,
            reply: russh::server::ChannelOpenHandle,
            _: &mut Session,
        ) -> Result<(), Self::Error> {
            if self.filesystem.take_channel_open_failure() {
                let filesystem = self.filesystem.clone();
                let channels = Arc::clone(&self.channels);
                tokio::spawn(async move {
                    if !filesystem.permit_channel_open().await {
                        return;
                    }
                    let id = channel.id();
                    channels.lock().await.insert(id, channel);
                    reply.accept().await;
                    filesystem.record_channel_open_accepted();
                });
                return Ok(());
            }
            self.channels.lock().await.insert(channel.id(), channel);
            reply.accept().await;
            Ok(())
        }
    };
}

pub(super) use channel_open_handler;
