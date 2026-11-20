use crate::constants::{
    AMBIGUOUS_TERMINAL, CATALOG_TIMEOUT, NO_MATCHING_TERMINAL, TERMINAL_TEXT_NO_RUNTIME,
    TERMINAL_TEXT_UNSUPPORTED, UNEXPECTED_TEXT_REPLY,
};
use std::io::{self, Write};
use yolo_hub_rpc_protocol::{Command, Method, Reply};
use yolo_hub_terminal_protocol::{
    ReadTerminalTextFields, ReadTerminalTextRequest, TERMINAL_PROTOCOL_SCHEMA_VERSION,
};
use yolo_tui_app::Options;

pub(crate) async fn run(options: &Options, selector: String, json: bool) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + CATALOG_TIMEOUT;
    let mut client = options.endpoint.connect(options.hub, deadline).await?;
    anyhow::ensure!(
        client.hello().methods.contains(&Method::ReadTerminalText),
        TERMINAL_TEXT_UNSUPPORTED
    );
    let catalog = yolo_hub_catalog_client::read_with_archived(&mut client, deadline, true).await?;
    let matches: Vec<_> = catalog
        .terminals
        .iter()
        .filter(|terminal| terminal.matches_selector(&selector))
        .collect();
    let terminal = match matches.as_slice() {
        [] => anyhow::bail!(NO_MATCHING_TERMINAL),
        [terminal] => *terminal,
        _ => anyhow::bail!(AMBIGUOUS_TERMINAL),
    };
    let target = terminal
        .fields()
        .runtime
        .ok_or_else(|| anyhow::anyhow!(TERMINAL_TEXT_NO_RUNTIME))?
        .target;
    let request = ReadTerminalTextRequest::new(ReadTerminalTextFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        target,
    })
    .map_err(|error| anyhow::anyhow!(error))?;
    let reply = client
        .call(
            client.hello().hub_id,
            Command::ReadTerminalText { request },
            deadline,
        )
        .await?;
    let Reply::TerminalText { text } = reply else {
        anyhow::bail!(UNEXPECTED_TEXT_REPLY);
    };
    let fields = text.fields();
    let mut output = io::stdout().lock();
    if json {
        serde_json::to_writer_pretty(&mut output, fields)?;
        writeln!(output)?;
    } else {
        output.write_all(fields.text.as_bytes())?;
        if !fields.text.ends_with('\n') {
            writeln!(output)?;
        }
    }
    Ok(())
}
