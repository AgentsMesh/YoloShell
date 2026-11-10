use super::constants::*;
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use super::server_channel_open::channel_open_handler;
use super::server_command::contains;
use super::server_command_cleanup::command_cleanup_handlers;
use super::server_hub_peer::hub_peer_handler;
use super::sftp_server::MemorySftp;
use crate::remote::constants::{SFTP_SUBSYSTEM, SSH_EXTENDED_DATA_STDERR, SUCCESS_EXIT_STATUS};
use russh::server::{Auth, Msg, Session};
use russh::{Channel, ChannelId, ChannelOpenFailure};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct MemorySshServer {
    channels: Arc<Mutex<HashMap<ChannelId, Channel<Msg>>>>,
    accepted_key: russh::keys::PublicKey,
    filesystem: MemoryFilesystem,
    hub_peers: std::collections::HashSet<ChannelId>,
}

impl MemorySshServer {
    pub fn with_filesystem(
        accepted_key: russh::keys::PublicKey,
        filesystem: MemoryFilesystem,
    ) -> Self {
        Self {
            channels: Arc::new(Mutex::new(HashMap::new())),
            accepted_key,
            filesystem,
            hub_peers: Default::default(),
        }
    }
}

impl russh::server::Handler for MemorySshServer {
    type Error = russh::Error;

    command_cleanup_handlers!();

    async fn auth_publickey(
        &mut self,
        _: &str,
        key: &russh::keys::PublicKey,
    ) -> Result<Auth, Self::Error> {
        Ok(if key == &self.accepted_key {
            Auth::Accept
        } else {
            Auth::Reject {
                proceed_with_methods: None,
                partial_success: false,
            }
        })
    }

    async fn auth_openssh_certificate(
        &mut self,
        user: &str,
        certificate: &russh::keys::Certificate,
    ) -> Result<Auth, Self::Error> {
        Ok(
            if user == MEMORY_USER && certificate.public_key() == self.accepted_key.key_data() {
                Auth::Accept
            } else {
                Auth::reject()
            },
        )
    }

    channel_open_handler!();
    hub_peer_handler!();

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        command: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if self.filesystem.take_exec_rejection() {
            session.channel_failure(channel)?;
            return Ok(());
        }
        if self
            .filesystem
            .take_failure(FailurePoint::ExecFlowControlBeforeApproval)
        {
            session.data(channel, UNEXPECTED_CHANNEL_DATA)?;
            session.xon_xoff_request(channel, PREAPPROVAL_FLOW_CONTROL_ENABLED)?;
            return Ok(());
        }
        if self
            .filesystem
            .take_failure(FailurePoint::ExecDataWithoutApproval)
        {
            session.data(channel, UNEXPECTED_CHANNEL_DATA)?;
            return Ok(());
        }
        if self
            .filesystem
            .take_failure(FailurePoint::ExecDataBeforeApproval)
        {
            session.data(channel, APPROVAL_COMMAND_PREFIX)?;
        }
        if self
            .filesystem
            .take_failure(FailurePoint::ExecEofThenDataBeforeApproval)
        {
            session.eof(channel)?;
            session.data(channel, POST_EOF_COMMAND_BYTES)?;
        }
        session.channel_success(channel)?;
        if super::server_identity::reply(&self.filesystem, command, channel, session)? {
            return Ok(());
        }
        if super::server_command::runtime(command) {
            let channel = self.channels.lock().await.remove(&channel).unwrap();
            tokio::spawn(async move {
                let _ = yolo_capnp_rpc::serve(channel.into_stream(), yolo_capnp_rpc::Protocol::Endpoint,
                    Arc::new(super::server_protocol::RuntimeService)).await;
            });
            return Ok(());
        }
        if contains(command, HANG_PROGRAM.as_bytes()) {
            self.filesystem.record_live_command();
            return Ok(());
        }
        if contains(command, PARTIAL_PROGRAM.as_bytes()) {
            self.filesystem.record_live_command();
            session.data(channel, PARTIAL_COMMAND_OUTPUT)?;
            self.filesystem.record_command_output();
            return Ok(());
        }
        let stdout = super::server_command::output(&self.filesystem, command);
        session.data(channel, stdout)?;
        session.extended_data(channel, SSH_EXTENDED_DATA_STDERR, COMMAND_STDERR)?;
        session.exit_status_request(channel, SUCCESS_EXIT_STATUS)?;
        session.eof(channel)?;
        session.close(channel)
    }

    async fn data(
        &mut self,
        _channel: ChannelId,
        _data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn subsystem_request(
        &mut self,
        channel: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name != SFTP_SUBSYSTEM {
            return session.channel_failure(channel);
        }
        if self.filesystem.take_confirmed_open_failure() {
            session.channel_success(channel)?;
            session.channel_open_failure(
                channel,
                ChannelOpenFailure::AdministrativelyProhibited,
                HOSTILE_OPEN_FAILURE_DESCRIPTION,
                HOSTILE_OPEN_FAILURE_LANGUAGE,
            )?;
            return Ok(());
        }
        if self.filesystem.take_transport_failure() {
            return Ok(());
        }
        let channel_stream = self.channels.lock().await.remove(&channel).unwrap();
        session.channel_success(channel)?;
        let filesystem = self.filesystem.clone();
        tokio::spawn(async move {
            russh_sftp::server::run(channel_stream.into_stream(), MemorySftp::new(filesystem))
                .await;
        });
        Ok(())
    }
}
