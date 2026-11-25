use clap::Subcommand;
use yolo_tui_app::Options;

/// Terminal-catalogue operations. These address Hub-owned PTYs, never the
/// client that displays them.
#[derive(Debug, Subcommand)]
pub(crate) enum TerminalCommand {
    /// List terminals visible from the selected Hub.
    #[command(group(crate::arguments::connection_group()))]
    List {
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
        /// Include archived terminals from history.
        #[arg(long)]
        all: bool,
    },
    /// Print the visible screen text of a terminal (non-authoritative).
    #[command(group(crate::arguments::connection_group()))]
    Screen {
        /// Terminal id, address, or unique name.
        terminal: String,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Archive an exited terminal; its history keeps the row.
    #[command(group(crate::arguments::connection_group()))]
    Archive {
        /// Terminal id, address, or unique name.
        terminal: String,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// End a terminal and every program running in it.
    #[command(group(crate::arguments::connection_group()))]
    Terminate {
        /// Terminal id, address, or unique name.
        terminal: String,
        /// Kill programs that do not exit after a graceful request.
        #[arg(long)]
        force: bool,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
}

impl TerminalCommand {
    pub(crate) async fn run(self, options: &Options) -> anyhow::Result<()> {
        match self {
            Self::List { json, all } => crate::listing::run(options, json, all).await,
            Self::Screen { terminal, json } => crate::screen::run(options, terminal, json).await,
            Self::Archive { .. } | Self::Terminate { .. } => {
                crate::lifecycle::run(options, self).await
            }
        }
    }
}
