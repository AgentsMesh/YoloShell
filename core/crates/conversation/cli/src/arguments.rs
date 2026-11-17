use crate::constants::{
    AGENT_OPTION, HUB_ARGUMENT, LOCAL_PROFILE_ARGUMENT, PROGRAM, SOCKET_ARGUMENT,
};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use yolo_hub_identity::{AgentId, HubId};

#[derive(Debug, Parser)]
#[command(
    name = PROGRAM,
    about = "Read native agent conversations through the public Hub protocol"
)]
pub(crate) struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    Conversation(Options),
}

#[derive(Debug, Args)]
pub(crate) struct Options {
    /// Existing local Hub entry socket; supply together with --hub.
    #[arg(long = SOCKET_ARGUMENT, requires = HUB_ARGUMENT)]
    pub socket: Option<PathBuf>,
    /// Identity of the Hub accepting this client connection.
    #[arg(long = HUB_ARGUMENT, requires = SOCKET_ARGUMENT)]
    pub hub: Option<HubId>,
    /// Local profile file; defaults to the current user profile.
    #[arg(long = LOCAL_PROFILE_ARGUMENT, conflicts_with_all = [SOCKET_ARGUMENT, HUB_ARGUMENT])]
    pub local_profile: Option<PathBuf>,
    #[command(subcommand)]
    pub action: Action,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Action {
    /// Resolve the current agent execution to its native conversation.
    Resolve {
        #[arg(long)]
        agent: AgentId,
        #[arg(long)]
        origin: Option<HubId>,
    },
    /// Read the latest page for --agent, or a typed read request from stdin.
    Read {
        #[arg(long)]
        agent: Option<AgentId>,
        #[arg(long, requires = AGENT_OPTION)]
        origin: Option<HubId>,
    },
    /// Read a typed conditional request from stdin and wait once for a change.
    Watch {
        #[arg(long, default_value_t = yolo_conversation_protocol::MAX_WATCH_WAIT_MS,
            value_parser = clap::value_parser!(u32).range(0..=i64::from(yolo_conversation_protocol::MAX_WATCH_WAIT_MS)))]
        wait_ms: u32,
    },
}
