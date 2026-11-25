use crate::constants::{ARCHIVED_STATE, CATALOG_TIMEOUT, TERMINAL_HEADER};
use std::io::{self, Write};
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_tui_app::Options;

pub(crate) async fn run(options: &Options, json: bool, all: bool) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + CATALOG_TIMEOUT;
    let mut client = options.endpoint.connect(options.hub, deadline).await?;
    let catalog = yolo_hub_catalog_client::read_with_archived(&mut client, deadline, all).await?;
    let mut output = io::stdout().lock();
    if json {
        serde_json::to_writer_pretty(&mut output, &catalog.terminals)?;
        writeln!(output)?;
    } else {
        writeln!(output, "{TERMINAL_HEADER}")?;
        for terminal in catalog.terminals {
            let fields = terminal.fields();
            writeln!(
                output,
                "{}\t{}\t{}",
                fields.address,
                state(&terminal),
                fields.name
            )?;
        }
    }
    Ok(())
}

pub(crate) fn state(terminal: &TerminalSummary) -> String {
    let fields = terminal.fields();
    if fields.archived {
        ARCHIVED_STATE.to_owned()
    } else {
        fields
            .runtime
            .map(|runtime| format!("{:?}", runtime.status))
            .unwrap_or_default()
    }
}
