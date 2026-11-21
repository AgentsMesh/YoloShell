use crate::constants::{
    CATALOG_TIMEOUT, MAX_SNAPSHOT_LIMIT, PLATFORM_SNAPSHOT_UNSUPPORTED, UNEXPECTED_SNAPSHOT_REPLY,
};
use std::io::{self, Write};
use yolo_hub_rpc_protocol::{
    Command, GetPlatformSnapshotFields, GetPlatformSnapshotRequest, Method,
    PLATFORM_SNAPSHOT_SCHEMA_VERSION, Reply,
};
use yolo_tui_app::Options;

pub(crate) async fn snapshot(options: &Options, json: bool, limit: u16) -> anyhow::Result<()> {
    anyhow::ensure!(
        (1..=MAX_SNAPSHOT_LIMIT).contains(&limit),
        "snapshot limit must be between 1 and {MAX_SNAPSHOT_LIMIT}"
    );
    let deadline = tokio::time::Instant::now() + CATALOG_TIMEOUT;
    let mut client = options.endpoint.connect(options.hub, deadline).await?;
    anyhow::ensure!(
        client
            .hello()
            .methods
            .contains(&Method::GetPlatformSnapshot),
        PLATFORM_SNAPSHOT_UNSUPPORTED
    );
    let request = GetPlatformSnapshotRequest::new(GetPlatformSnapshotFields {
        schema_version: PLATFORM_SNAPSHOT_SCHEMA_VERSION,
        terminal_limit: limit,
        agent_limit: limit,
        run_limit: limit,
        workspace_limit: limit,
    })
    .map_err(|error| anyhow::anyhow!(error))?;
    let hub = client.hello().hub_id;
    let reply = client
        .call(hub, Command::GetPlatformSnapshot { request }, deadline)
        .await?;
    let Reply::PlatformSnapshot { snapshot } = reply else {
        anyhow::bail!(UNEXPECTED_SNAPSHOT_REPLY);
    };
    let fields = snapshot.fields();
    let mut output = io::stdout().lock();
    if json {
        serde_json::to_writer_pretty(&mut output, fields)?;
        writeln!(output)?;
    } else {
        writeln!(
            output,
            "hubs={} terminals={} agents={} runs={} workspaces={}",
            fields.hubs.len(),
            fields.terminals.len(),
            fields.agents.len(),
            fields.runs.len(),
            fields.workspaces.len()
        )?;
    }
    Ok(())
}
