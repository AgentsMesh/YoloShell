use crate::constants::DEFAULT_EVENTS_LIMIT;
use clap::Subcommand;
use uuid::Uuid;
use yolo_tui_app::Options;

#[derive(Debug, Subcommand)]
pub(crate) enum PlatformCommand {
    /// Print hubs, terminals, agents, runs and workspaces in one document.
    #[command(group(crate::arguments::connection_group()))]
    Snapshot {
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
        /// Maximum entries per section (1-128).
        #[arg(long, default_value_t = crate::constants::DEFAULT_SNAPSHOT_LIMIT)]
        limit: u16,
    },
    /// Print sequenced platform events after a cursor.
    #[command(group(crate::arguments::connection_group()))]
    Events {
        /// Stream epoch returned by an earlier call.
        #[arg(long, requires = "after")]
        epoch: Option<Uuid>,
        /// Last sequence already handled; requires --epoch.
        #[arg(long, requires = "epoch")]
        after: Option<u64>,
        /// Maximum events to return (1-256).
        #[arg(long, default_value_t = DEFAULT_EVENTS_LIMIT)]
        limit: u16,
        /// Keep waiting for new events until interrupted.
        #[arg(long)]
        follow: bool,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
}

pub(crate) async fn run(options: &Options, command: PlatformCommand) -> anyhow::Result<()> {
    match command {
        PlatformCommand::Snapshot { json, limit } => {
            crate::platform_snapshot::snapshot(options, json, limit).await
        }
        PlatformCommand::Events {
            epoch,
            after,
            limit,
            follow,
            json,
        } => crate::platform_events::events(options, epoch, after, limit, follow, json).await,
    }
}
