use crate::constants::*;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use yolo_hub_identity::HubId;

mod command_args;
pub use command_args::{
    AskArgs, LabelArgs, PromptArgs, PromptExecArgs, RunState, RunsArgs, StartArgs, StopArgs,
};

#[derive(Debug, Parser)]
#[command(
    name = PROGRAM,
    about = "Inspect and control a specific agent through the public Hub protocol"
)]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Agent(Options),
}

#[derive(Debug, Args)]
pub struct Options {
    /// Existing local Hub entry socket; supply together with --hub.
    #[arg(long, global = true, requires = HUB_ARGUMENT, help_heading = "Connection")]
    pub socket: Option<PathBuf>,
    /// Identity of the Hub accepting this client connection.
    #[arg(long, global = true, requires = SOCKET_ARGUMENT, help_heading = "Connection")]
    pub hub: Option<HubId>,
    /// Local profile file; defaults to the current user profile.
    #[arg(long, global = true, conflicts_with_all = [SOCKET_ARGUMENT, HUB_ARGUMENT], help_heading = "Connection")]
    pub local_profile: Option<PathBuf>,
    #[command(subcommand)]
    pub command: AgentCommand,
}

#[derive(Debug, Subcommand)]
pub enum AgentCommand {
    /// List observed agents.
    List(ListArgs),
    /// List durable agent runs or show one by name/UUID.
    Runs(RunsArgs),
    /// Set or remove labels on an agent run.
    Label(LabelArgs),
    /// Show the agent running in the terminal this command runs in.
    Current(OutputArgs),
    /// Create a terminal and submit one command, waiting for an agent binding.
    Start(StartArgs),
    /// Stop an agent run by ending its terminal.
    Stop(StopArgs),
    /// Show one agent's current observation.
    Get {
        /// Agent id, terminal id, or unique terminal name.
        #[arg(required_unless_present = "current", conflicts_with = "current")]
        selector: Option<String>,
        /// Resolve the agent from YOLO_TERMINAL_ID in the calling shell.
        #[arg(long)]
        current: bool,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Deliver a prompt through the terminal writer; does not wait for the turn.
    Send(PromptArgs),
    /// Deliver a prompt, then poll until the turn ends or --timeout expires.
    Ask(AskArgs),
    /// Submit one prompt atomically and optionally wait for the turn to end.
    Prompt(PromptExecArgs),
    /// Send an interrupt to the agent.
    Interrupt {
        /// Agent id, terminal id, or unique terminal name.
        #[arg(required_unless_present = "current", conflicts_with = "current")]
        selector: Option<String>,
        /// Resolve the agent from YOLO_TERMINAL_ID in the calling shell.
        #[arg(long)]
        current: bool,
        /// Take writer control from another client instead of failing.
        #[arg(long)]
        take_control: bool,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Answer the agent's pending permission or choice interaction.
    #[command(allow_missing_positional = true)]
    Respond {
        /// Agent id, terminal id, or unique terminal name.
        #[arg(required_unless_present = "current", conflicts_with = "current")]
        selector: Option<String>,
        /// Resolve the agent from YOLO_TERMINAL_ID in the calling shell.
        #[arg(long)]
        current: bool,
        /// Choice id from the pending interaction, such as allow or deny.
        choice: String,
        /// Take writer control from another client instead of failing.
        #[arg(long)]
        take_control: bool,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Read the agent's latest native conversation page.
    Read {
        /// Agent id, terminal id, or unique terminal name.
        #[arg(required_unless_present = "current", conflicts_with = "current")]
        selector: Option<String>,
        /// Resolve the agent from YOLO_TERMINAL_ID in the calling shell.
        #[arg(long)]
        current: bool,
        /// Maximum entries in the returned page.
        #[arg(
            long,
            default_value_t = DEFAULT_PAGE_ENTRIES,
            value_parser = clap::value_parser!(u32).range(1..=i64::from(MAX_PAGE_ENTRIES))
        )]
        limit: u32,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Wait until the agent reaches one of the given states.
    Wait(WaitArgs),
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Include exited and stale agents from history. Freshness describes the
    /// observation, so a historical row can still be current for its terminal.
    #[arg(long)]
    pub all: bool,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct WaitArgs {
    /// Agent ids, terminal ids, or unique terminal names; the wait matches any
    /// of them and all must belong to the same Hub.
    #[arg(
        required_unless_present_any = ["current", "runs"],
        conflicts_with_all = ["current", "runs"],
        num_args = 1..,
        value_name = "SELECTOR"
    )]
    pub selectors: Vec<String>,
    /// Durable run names or run UUIDs whose current executions are waited on.
    #[arg(long = "run", value_name = "RUN", conflicts_with = "selectors")]
    pub runs: Vec<String>,
    /// Resolve the agent from YOLO_TERMINAL_ID in the calling shell.
    #[arg(long, conflicts_with = "runs")]
    pub current: bool,
    /// State to wait for; repeat to wait for any of several states.
    #[arg(
        long = "until",
        value_name = "STATE",
        required = true,
        value_parser = [
            STATE_IDLE, STATE_WORKING, STATE_WAITING_FOR_INPUT, "prompt",
            STATE_WAITING_FOR_APPROVAL, "approval", STATE_EXITED,
            STATE_UNREACHABLE, STATE_STALE, STATE_PROMPTABLE, STATE_INTERRUPTIBLE,
            STATE_RESPONDABLE
        ]
    )]
    pub until: Vec<String>,
    /// Seconds to wait before giving up.
    #[arg(
        long,
        default_value_t = DEFAULT_TIMEOUT_SECS,
        value_parser = clap::value_parser!(u64).range(1..=MAX_TIMEOUT_SECS)
    )]
    pub timeout: u64,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct OutputArgs {
    /// Print one machine-readable JSON document.
    #[arg(long)]
    pub json: bool,
}
