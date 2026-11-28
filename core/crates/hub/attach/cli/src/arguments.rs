use crate::constants::*;
use clap::Parser;
use std::path::PathBuf;
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::{TerminalSize, WriterPrincipal};
use yolo_hub_terminal_relay::{ControlIntent, Options};

#[derive(Debug, Parser)]
#[command(name = PROGRAM, version)]
pub struct Arguments {
    #[arg(long = SOCKET_OPTION)]
    pub socket: PathBuf,
    #[arg(long = HUB_OPTION)]
    pub hub: HubId,
    #[arg(long = ORIGIN_OPTION)]
    pub origin: HubId,
    #[arg(long = TERMINAL_OPTION)]
    pub terminal_id: TerminalId,
    #[arg(long = EPOCH_OPTION)]
    pub runtime_epoch: RuntimeEpoch,
    #[arg(long = HOLDER_OPTION)]
    pub holder_id: uuid::Uuid,
    #[arg(long = STATUS_OPTION)]
    pub status_fd: Option<libc::c_int>,
    #[arg(long = COMMAND_OPTION, requires = LAUNCH_FIELD)]
    pub command_fd: Option<libc::c_int>,
    #[arg(long = LAUNCH_OPTION, requires = COMMAND_FIELD)]
    pub launch_id: Option<uuid::Uuid>,
    #[arg(long = OBSERVE_OPTION, conflicts_with = TAKEOVER_FIELD)]
    pub observe: bool,
    #[arg(long = TAKEOVER_OPTION, conflicts_with = OBSERVE_FIELD)]
    pub take_control: bool,
}
impl Arguments {
    pub(crate) fn validate_channels(&self) -> Result<(), crate::CliError> {
        match (self.command_fd, self.launch_id) {
            (None, None) => Ok(()),
            (Some(fd), Some(launch))
                if fd > libc::STDERR_FILENO && self.status_fd != Some(fd) && !launch.is_nil() =>
            {
                Ok(())
            }
            _ => Err(crate::CliError::CommandDescriptor),
        }
    }
    pub fn options(&self, size: Option<TerminalSize>) -> Options {
        Options {
            socket: self.socket.clone(),
            entry: self.hub,
            target: RuntimeTarget {
                address: TerminalAddress {
                    hub_id: self.origin,
                    terminal_id: self.terminal_id,
                },
                runtime_epoch: self.runtime_epoch,
            },
            principal: WriterPrincipal {
                client_id: self.holder_id,
                controller_instance_id: uuid::Uuid::new_v4(),
            },
            intent: if self.observe {
                ControlIntent::Observe
            } else if self.take_control {
                ControlIntent::TakeControl
            } else {
                ControlIntent::Acquire
            },
            size,
        }
    }
}
