macro_rules! command_cleanup_handlers {
    () => {
        fn signal(
            &mut self,
            _: russh::ChannelId,
            signal: russh::Sig,
            _: &mut russh::server::Session,
        ) -> impl Future<Output = Result<(), Self::Error>> + Send {
            self.filesystem.record_command_signal(&signal);
            async { Ok(()) }
        }

        fn channel_close(
            &mut self,
            channel: russh::ChannelId,
            _: &mut russh::server::Session,
        ) -> impl Future<Output = Result<(), Self::Error>> + Send {
            self.filesystem.record_command_close();
            let hub_peer = self.hub_peers.remove(&channel);
            self.filesystem.record_hub_peer_close(channel, hub_peer);
            let channels = Arc::clone(&self.channels);
            async move {
                channels.lock().await.remove(&channel);
                Ok(())
            }
        }
    };
}

pub(super) use command_cleanup_handlers;
