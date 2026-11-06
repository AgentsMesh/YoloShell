use clap::Subcommand;

/// Local client operations. A client is one attached TUI process; it owns no
/// terminal and detaching never stops the shells it displays.
#[derive(Debug, Subcommand)]
pub(crate) enum ClientCommand {
    /// List local terminal clients connected to the selected Hub.
    #[command(group(crate::arguments::connection_group()))]
    List {
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Disconnect a local terminal client; keep its shells and agents running.
    #[command(group(crate::arguments::connection_group()))]
    Detach {
        /// Local client id; omit to choose from the attached clients.
        client: Option<uuid::Uuid>,
    },
}
