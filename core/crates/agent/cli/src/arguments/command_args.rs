use super::OutputArgs;
use crate::constants::{
    DEFAULT_PROMPT_WAIT_MS, DEFAULT_RUN_PAGE, DEFAULT_START_TIMEOUT_SECS, DEFAULT_TIMEOUT_SECS,
    MAX_PROMPT_WAIT_MS, MAX_RUN_PAGE, MAX_TIMEOUT_SECS,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct StartArgs {
    /// Provider id to launch through the Hub's closed launch plan.
    #[arg(long)]
    pub provider: String,
    /// Label for the new terminal.
    #[arg(long)]
    pub name: Option<String>,
    /// Working directory; defaults to the caller's current directory.
    #[arg(long)]
    pub cwd: Option<String>,
    /// Workspace id; the Hub injects YOLO_HUB_ID/YOLO_WORKSPACE_ID.
    #[arg(long)]
    pub workspace: Option<uuid::Uuid>,
    /// Inherit the caller terminal's Hub and workspace in a new terminal.
    #[arg(long, conflicts_with = "workspace")]
    pub current: bool,
    /// Parent run name or UUID; recorded as orchestration lineage.
    #[arg(long, value_name = "RUN")]
    pub parent: Option<String>,
    /// Seconds to wait for the agent binding before reporting awaiting.
    #[arg(
        long,
        default_value_t = DEFAULT_START_TIMEOUT_SECS,
        value_parser = clap::value_parser!(u64).range(1..=MAX_TIMEOUT_SECS)
    )]
    pub timeout: u64,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct StopArgs {
    /// Run name or run UUID.
    pub run: String,
    /// Kill programs that do not exit after a graceful request.
    #[arg(long)]
    pub force: bool,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct PromptArgs {
    /// Agent id, terminal id, or unique terminal name.
    #[arg(required_unless_present = "current", conflicts_with = "current")]
    pub selector: Option<String>,
    /// Resolve the agent from YOLO_TERMINAL_ID in the calling shell.
    #[arg(long)]
    pub current: bool,
    /// Prompt text; the agent's editor content is appended before submission.
    #[arg(required_unless_present = "stdin", conflicts_with = "stdin")]
    pub prompt: Option<String>,
    /// Read the prompt from standard input instead of an argument.
    #[arg(long)]
    pub stdin: bool,
    /// Take writer control from another client instead of failing.
    #[arg(long)]
    pub take_control: bool,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct PromptExecArgs {
    /// Agent id, terminal id, or unique terminal name.
    #[arg(required_unless_present = "current", conflicts_with = "current")]
    pub selector: Option<String>,
    /// Resolve the agent from YOLO_TERMINAL_ID in the calling shell.
    #[arg(long)]
    pub current: bool,
    /// Prompt text; the agent's editor content is appended before submission.
    #[arg(required_unless_present = "stdin", conflicts_with = "stdin")]
    pub prompt: Option<String>,
    /// Read the prompt from standard input instead of an argument.
    #[arg(long)]
    pub stdin: bool,
    /// Take writer control from another client instead of failing.
    #[arg(long)]
    pub take_control: bool,
    /// Wait for the turn to leave the working state after submission.
    #[arg(long)]
    pub wait_turn: bool,
    /// Bound for the post-submission turn wait.
    #[arg(
        long,
        default_value_t = DEFAULT_PROMPT_WAIT_MS,
        value_parser = clap::value_parser!(u32).range(1..=i64::from(MAX_PROMPT_WAIT_MS))
    )]
    pub wait_ms: u32,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct AskArgs {
    /// Agent id, terminal id, or unique terminal name.
    #[arg(required_unless_present = "current", conflicts_with = "current")]
    pub selector: Option<String>,
    /// Resolve the agent from YOLO_TERMINAL_ID in the calling shell.
    #[arg(long)]
    pub current: bool,
    /// Prompt text; the agent's editor content is appended before submission.
    #[arg(required_unless_present = "stdin", conflicts_with = "stdin")]
    pub prompt: Option<String>,
    /// Read the prompt from standard input instead of an argument.
    #[arg(long)]
    pub stdin: bool,
    /// Take writer control from another client instead of failing.
    #[arg(long)]
    pub take_control: bool,
    /// Seconds to wait for the agent to leave its working state.
    #[arg(
        long,
        default_value_t = DEFAULT_TIMEOUT_SECS,
        value_parser = clap::value_parser!(u64).range(1..=MAX_TIMEOUT_SECS)
    )]
    pub timeout: u64,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum RunState {
    Starting,
    Launched,
    Stopped,
}

#[derive(Debug, Args)]
pub struct LabelArgs {
    /// Run name or UUID.
    pub run: String,
    /// Label as key=value; repeatable.
    #[arg(long = "label", value_name = "KEY=VALUE")]
    pub labels: Vec<String>,
    /// Remove a label key; repeatable.
    #[arg(long)]
    pub remove: Vec<String>,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct RunsArgs {
    /// Show one run by name or UUID instead of listing.
    #[arg(
        long,
        value_name = "RUN",
        conflicts_with_all = ["state", "provider", "parent", "name", "label", "after"]
    )]
    pub show: Option<String>,
    /// Filter by durable state; repeat for any of several.
    #[arg(long, value_enum, conflicts_with_all = ["provider", "parent", "name", "label"])]
    pub state: Vec<RunState>,
    /// Filter by provider id.
    #[arg(long, conflicts_with_all = ["state", "parent", "name", "label"])]
    pub provider: Option<String>,
    /// Filter by parent run UUID (single run queries only by name/UUID).
    #[arg(long, conflicts_with_all = ["state", "provider", "name", "label"])]
    pub parent: Option<uuid::Uuid>,
    /// Filter by run name or alias.
    #[arg(long, conflicts_with_all = ["state", "provider", "parent", "label"])]
    pub name: Option<String>,
    /// Filter by run label as key=value.
    #[arg(long, conflicts_with_all = ["state", "provider", "parent", "name"])]
    pub label: Option<String>,
    /// Resume after this run UUID.
    #[arg(long)]
    pub after: Option<uuid::Uuid>,
    /// Maximum runs to return.
    #[arg(
        long,
        default_value_t = DEFAULT_RUN_PAGE,
        value_parser = clap::value_parser!(u16).range(1..=i64::from(MAX_RUN_PAGE))
    )]
    pub limit: u16,
    #[command(flatten)]
    pub output: OutputArgs,
}
