use clap::Subcommand;
use yolo_local_profile::LocalProfile;
use yolo_tui_service::FormatOptions;

/// Local Host and Hub lifecycle. These commands never create, attach or stop a
/// terminal; detaching a client is `yolo client detach`.
#[derive(Debug, Subcommand)]
pub(crate) enum ServerCommand {
    /// Start the local Host and Hub.
    #[command(group(crate::arguments::connection_group()))]
    Start(FormatOptions),
    /// Stop the local Host and Hub; terminals and PTYs keep running.
    #[command(group(crate::arguments::connection_group()))]
    Stop(FormatOptions),
    /// Restart the local Host and Hub.
    #[command(group(crate::arguments::connection_group()))]
    Restart(FormatOptions),
    /// Show installed and running local service status.
    #[command(group(crate::arguments::connection_group()))]
    Status(FormatOptions),
    /// Print installed and running local service versions.
    #[command(group(crate::arguments::connection_group()))]
    Version(FormatOptions),
}

impl ServerCommand {
    pub(crate) async fn run(self, profile: &LocalProfile) -> anyhow::Result<()> {
        match self {
            Self::Start(options) => yolo_tui_service::start_command(profile, options).await,
            Self::Stop(options) => yolo_tui_service::stop_command(profile, options).await,
            Self::Restart(options) => yolo_tui_service::restart_command(profile, options).await,
            Self::Status(options) => yolo_tui_service::status_command(profile, options).await,
            Self::Version(options) => yolo_tui_service::version_command(profile, options).await,
        }
    }
}
