use super::broker_command_fixture::BrokerCommandCleanupFixture;
use super::constants::{COMMAND_STDOUT, NORMAL_PROGRAM};
use super::memory_fs::FailurePoint;
use crate::remote::{CommandPolicy, CommandRequest, MultiplexedSshConnection};
use std::net::{Ipv4Addr, SocketAddr};

const DYNAMIC_PORT: u16 = 0;

impl BrokerCommandCleanupFixture {
    pub async fn enable_hub_peer(&self) -> tokio::net::TcpListener {
        let listener = tokio::net::TcpListener::bind(SocketAddr::new(
            Ipv4Addr::LOCALHOST.into(),
            DYNAMIC_PORT,
        ))
        .await
        .unwrap();
        self.memory
            .filesystem
            .set_hub_peer_target(listener.local_addr().unwrap());
        listener
    }
    pub fn delay_next_channel(&self) {
        self.memory.filesystem.fail_once(FailurePoint::ChannelOpen);
    }
    pub async fn wait_pending_channel(&self) {
        self.memory.filesystem.wait_channel_open_pending().await;
    }
    pub fn release_pending_channel(&self) {
        self.memory.filesystem.release_channel_open();
    }
    pub async fn wait_accepted_channel(&self) {
        self.memory.filesystem.wait_channel_open_accepted().await;
    }
    pub async fn assert_ssh_healthy(&self) {
        let command = CommandPolicy::new([NORMAL_PROGRAM.into()])
            .unwrap()
            .authorize(CommandRequest::new(NORMAL_PROGRAM, Vec::new()))
            .unwrap();
        let output = self.connection().execute(&command).await.unwrap();
        assert_eq!(output.value.stdout, COMMAND_STDOUT);
    }
    pub async fn settle(&self) {
        self.memory
            .connection
            .cleanup
            .settle(crate::remote::cleanup_settlement_timeout(
                super::constants::NORMAL_OPERATION_TIMEOUT,
            ))
            .await
            .unwrap();
    }
}
