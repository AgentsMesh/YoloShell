use super::super::RusshConnection;
use super::constants::{
    EXPECTED_COMMAND_CLEANUPS, EXPECTED_LIVE_COMMANDS, HANG_PROGRAM, NO_LIVE_COMMANDS,
};
use super::fixture::{MemoryConnection, connect_memory};
use std::sync::Arc;

pub const BROKER_COMMAND_CANCELLATION_PROGRAM: &str = HANG_PROGRAM;

pub struct BrokerCommandCleanupFixture {
    pub(super) memory: MemoryConnection,
}

impl BrokerCommandCleanupFixture {
    pub async fn connect() -> Self {
        Self {
            memory: connect_memory(false).await.unwrap(),
        }
    }

    pub fn connection(&self) -> Arc<RusshConnection> {
        Arc::clone(&self.memory.connection)
    }

    pub fn command_is_live(&self) -> bool {
        self.memory.filesystem.live_commands() == EXPECTED_LIVE_COMMANDS
    }

    pub fn command_cleanup_is_exact(&self) -> bool {
        self.memory.filesystem.command_signals() == EXPECTED_COMMAND_CLEANUPS
            && self.memory.filesystem.command_closes() == EXPECTED_COMMAND_CLEANUPS
            && self.memory.filesystem.live_commands() == NO_LIVE_COMMANDS
    }
}
