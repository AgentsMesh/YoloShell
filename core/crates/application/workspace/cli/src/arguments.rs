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
    /// Manage durable workspace metadata through the public Hub.
    Workspace(Options),
    /// Manage git worktrees through the public Hub.
    Worktree(WorktreeOptions),
}

#[derive(Debug, Args)]
pub struct WorktreeOptions {
    /// Existing local Hub entry socket; supply together with --hub.
    #[arg(long, global = true, help_heading = "Connection")]
    pub socket: Option<PathBuf>,
    /// Identity of the Hub accepting this client connection.
    #[arg(long, global = true, help_heading = "Connection")]
    pub hub: Option<yolo_hub_identity::HubId>,
    /// Local profile file; defaults to the current user profile.
    #[arg(long, global = true, help_heading = "Connection")]
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
    /// Unique name for the worktree workspace.
    #[arg(long)]
    pub name: String,
    /// Branch to create; defaults to yolo/<worktree-id>.
    #[arg(long)]
    pub branch: Option<String>,
    /// Commit-ish the new branch starts from.
    #[arg(long)]
    pub base: Option<String>,
    /// Required acknowledgement that repository git hooks execute code.
    #[arg(long, required = true)]
    pub trust_repository: bool,
    /// Print one JSON document.
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
    /// Print one JSON document.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct Options {
    /// Existing local Hub entry socket; supply together with --hub.
    #[arg(long, global = true, help_heading = "Connection")]
    pub socket: Option<PathBuf>,
    /// Identity of the Hub accepting this client connection.
    #[arg(long, global = true, help_heading = "Connection")]
    pub hub: Option<yolo_hub_identity::HubId>,
    /// Local profile file; defaults to the current user profile.
    #[arg(long, global = true, help_heading = "Connection")]
    pub local_profile: Option<PathBuf>,
    #[command(subcommand)]
    pub command: WorkspaceCommand,
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceCommand {
    /// Create a durable workspace.
    Create(CreateArgs),
    /// Rename, relabel or bind terminals in a workspace.
    Update(UpdateArgs),
    /// Show one workspace by id or unique name.
    Get(GetArgs),
    /// List workspaces after an optional cursor.
    List(ListArgs),
    /// List the durable terminal and run bindings of one workspace.
    Members(MembersArgs),
}

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Unique workspace name.
    #[arg(long)]
    pub name: String,
    /// Workspace root path on the Hub that owns the workspace.
    #[arg(long)]
    pub root: String,
    /// Label in key=value form; repeatable.
    #[arg(long = "label")]
    pub labels: Vec<String>,
    /// Print one JSON document.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Workspace id or unique name.
    pub selector: String,
    /// Revision the caller last observed; the default is the revision just read.
    #[arg(long)]
    pub expected_revision: Option<u64>,
    /// New unique workspace name.
    #[arg(long)]
    pub name: Option<String>,
    /// Label in key=value form; repeatable.
    #[arg(long = "label")]
    pub labels: Vec<String>,
    /// Bind an existing terminal id to this workspace.
    #[arg(long = "bind-terminal")]
    pub bind_terminals: Vec<uuid::Uuid>,
    /// Unbind a terminal id from this workspace.
    #[arg(long = "unbind-terminal")]
    pub unbind_terminals: Vec<uuid::Uuid>,
    /// Print one JSON document.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    /// Workspace id or unique name.
    pub selector: String,
    /// Print one JSON document.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Resume after this workspace id.
    #[arg(long)]
    pub after: Option<uuid::Uuid>,
    /// Maximum workspaces to return.
    #[arg(long, default_value_t = DEFAULT_LIMIT, value_parser = clap::value_parser!(u16).range(1..=MAX_LIMIT as i64))]
    pub limit: u16,
    /// Print one JSON document.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct MembersArgs {
    /// Workspace id or name.
    pub selector: String,
    /// Print one JSON document.
    #[arg(long)]
    pub json: bool,
}
