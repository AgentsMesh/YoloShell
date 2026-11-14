use crate::constants::*;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = PROGRAM, about = "Manage durable workspace metadata through the public Hub")]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Workspace(Options),
    Worktree(WorktreeOptions),
}

#[derive(Debug, Args)]
pub struct WorktreeOptions {
    /// Existing local Hub entry socket; supply together with --hub.
    #[arg(long, global = true)]
    pub socket: Option<PathBuf>,
    /// Identity of the Hub accepting this client connection.
    #[arg(long, global = true)]
    pub hub: Option<yolo_hub_identity::HubId>,
    /// Local profile file; defaults to the current user profile.
    #[arg(long, global = true)]
    pub local_profile: Option<PathBuf>,
    #[command(subcommand)]
    pub command: WorktreeCommand,
}

#[derive(Debug, Subcommand)]
pub enum WorktreeCommand {
    /// Create a git worktree of a parent workspace and register it.
    Create(WorktreeCreateArgs),
    /// Remove a registered worktree and its workspace record.
    Remove(WorktreeRemoveArgs),
    /// List worktree workspaces of a parent.
    List(WorktreeListArgs),
}

#[derive(Debug, Args)]
pub struct WorktreeCreateArgs {
    /// Parent workspace id or name.
    pub parent: String,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long)]
    pub base: Option<String>,
    /// Required acknowledgement that repository git hooks execute code.
    #[arg(long)]
    pub trust_repository: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct WorktreeRemoveArgs {
    /// Worktree workspace id or name.
    pub selector: String,
}

#[derive(Debug, Args)]
pub struct WorktreeListArgs {
    /// Parent workspace id or name.
    pub parent: String,
    #[arg(long)]
    pub after: Option<uuid::Uuid>,
    #[arg(long, default_value_t = DEFAULT_LIMIT, value_parser = clap::value_parser!(u16).range(1..=MAX_LIMIT as i64))]
    pub limit: u16,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct Options {
    /// Existing local Hub entry socket; supply together with --hub.
    #[arg(long, global = true)]
    pub socket: Option<PathBuf>,
    /// Identity of the Hub accepting this client connection.
    #[arg(long, global = true)]
    pub hub: Option<yolo_hub_identity::HubId>,
    /// Local profile file; defaults to the current user profile.
    #[arg(long, global = true)]
    pub local_profile: Option<PathBuf>,
    #[command(subcommand)]
    pub command: WorkspaceCommand,
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceCommand {
    Create(CreateArgs),
    Update(UpdateArgs),
    Get(GetArgs),
    List(ListArgs),
    /// List the durable terminal and run bindings of one workspace.
    Members(MembersArgs),
}

#[derive(Debug, Args)]
pub struct CreateArgs {
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub root: String,
    #[arg(long = "label")]
    pub labels: Vec<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    pub selector: String,
    #[arg(long)]
    pub expected_revision: u64,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long = "label")]
    pub labels: Vec<String>,
    /// Bind an existing terminal id to this workspace.
    #[arg(long = "bind-terminal")]
    pub bind_terminals: Vec<uuid::Uuid>,
    /// Unbind a terminal id from this workspace.
    #[arg(long = "unbind-terminal")]
    pub unbind_terminals: Vec<uuid::Uuid>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    pub selector: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Resume after this workspace id.
    #[arg(long)]
    pub after: Option<uuid::Uuid>,
    #[arg(long, default_value_t = DEFAULT_LIMIT, value_parser = clap::value_parser!(u16).range(1..=MAX_LIMIT as i64))]
    pub limit: u16,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct MembersArgs {
    /// Workspace id or name.
    pub selector: String,
    #[arg(long)]
    pub json: bool,
}
