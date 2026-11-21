use crate::constants::*;
use crate::output;
use anyhow::Result;
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;

pub(crate) async fn list(
    socket: &Path,
    entry: HubId,
    json: bool,
    deadline: Instant,
) -> Result<()> {
    let snapshot = super::read(socket, entry, deadline).await?;
    if json {
        println!("{}", output::list_json(&snapshot)?);
        return Ok(());
    }
    let machines = &snapshot.state.configuration.peers;
    if machines.is_empty() {
        println!("{MACHINE_LIST_EMPTY}");
        return Ok(());
    }
    println!("{MACHINE_LIST_HEADER}");
    for machine in machines {
        let (label, socket) = machine.dial.as_ref().map_or_else(
            || ("pinned".to_owned(), "-".to_owned()),
            |dial| {
                (
                    dial.ssh_target()
                        .or_else(|| dial.profile_id())
                        .unwrap_or_default()
                        .to_owned(),
                    dial.socket().display().to_string(),
                )
            },
        );
        let imports = if machine.consume { "yes" } else { "no" };
        let exports = machine
            .exports
            .iter()
            .map(|policy| policy.origin.to_string())
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{}\t{}\t{}\t{}\t{}",
            label,
            machine.identity.hub(),
            socket,
            imports,
            if exports.is_empty() {
                "-".into()
            } else {
                exports
            }
        );
    }
    println!("{MACHINE_LIST_LEGEND}");
    Ok(())
}
