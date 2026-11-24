use crate::{constants::*, socket::SocketState};
use russh::{Channel, client, keys::PublicKey};
use std::sync::Arc;

pub(crate) struct Handler {
    pub pin: PublicKey,
    pub socket: Arc<SocketState>,
}
impl Drop for Handler {
    fn drop(&mut self) {
        self.socket.close();
    }
}
impl client::Handler for Handler {
    type Error = russh::Error;
    async fn check_server_key(&mut self, key: &PublicKey) -> Result<bool, Self::Error> {
        Ok(key.key_data() == self.pin.key_data())
    }
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
