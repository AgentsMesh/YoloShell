use std::io::{self, Write};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_tui_app::Options;

const HEADER: &str = "LAYOUT_ID\tREVISION\tTABS\tPANES";
const INVALID_REPLY: &str = "Hub returned an unexpected layout listing";

pub(crate) async fn run(options: &Options, json: bool) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + crate::constants::CATALOG_TIMEOUT;
    let mut client = options.endpoint.connect(options.hub, deadline).await?;
    let Reply::Layouts { documents } = client
        .call(options.hub, Command::ListLayouts {}, deadline)
        .await?
    else {
        anyhow::bail!(INVALID_REPLY)
    };
    let mut output = io::stdout().lock();
    if json {
        serde_json::to_writer_pretty(&mut output, &documents)?;
        writeln!(output)?;
    } else {
        writeln!(output, "{HEADER}")?;
        for document in documents {
            writeln!(
                output,
                "{}\t{}\t{}\t{}",
                document.address.layout_id,
                document.revision,
                document.tab_count,
                document.pane_count
            )?;
        }
    }
    Ok(())
}
