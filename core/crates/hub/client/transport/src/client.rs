use crate::{HubPin, constants::*, custody::CloseState};
use russh::{Channel, client, keys::PublicKey};
use std::sync::Arc;

pub(crate) struct ClientHandler {
    pub pin: HubPin,
    pub transport: Arc<CloseState>,
}
impl Drop for ClientHandler {
    fn drop(&mut self) {
        self.transport.close();
    }
}
impl client::Handler for ClientHandler {
    type Error = anyhow::Error;
    async fn check_server_key(&mut self, key: &PublicKey) -> Result<bool, Self::Error> {
        Ok(self.pin.matches(key))
    }
    // russh accepts several reverse-channel types by default. Public clients
    // grant none of them; dropping each reply explicitly rejects the open.
    async fn server_channel_open_session(
        &mut self,
        _: Channel<client::Msg>,
        _: client::ChannelOpenHandle,
        _: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn server_channel_open_agent_forward(
        &mut self,
        _: Channel<client::Msg>,
        _: client::ChannelOpenHandle,
        _: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        _: Channel<client::Msg>,
        _: &str,
        _: u32,
        _: &str,
        _: u32,
        _: client::ChannelOpenHandle,
        _: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn server_channel_open_forwarded_streamlocal(
        &mut self,
        _: Channel<client::Msg>,
        _: &str,
        _: client::ChannelOpenHandle,
        _: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn server_channel_open_direct_tcpip(
        &mut self,
        _: Channel<client::Msg>,
        _: &str,
        _: u32,
        _: &str,
        _: u32,
        _: client::ChannelOpenHandle,
        _: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn server_channel_open_direct_streamlocal(
        &mut self,
        _: Channel<client::Msg>,
        _: &str,
        _: client::ChannelOpenHandle,
        _: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn server_channel_open_x11(
        &mut self,
        _: Channel<client::Msg>,
        _: &str,
        _: u32,
        _: client::ChannelOpenHandle,
        _: &mut client::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
pub(crate) fn config() -> Arc<client::Config> {
    Arc::new(client::Config {
        channel_buffer_size: SSH_CHANNEL_MESSAGES,
        window_size: SSH_WINDOW_BYTES,
        maximum_packet_size: SSH_PACKET_BYTES,
        keepalive_interval: Some(KEEPALIVE_INTERVAL),
        keepalive_max: MISSED_KEEPALIVES,
        ..Default::default()
    })
}
