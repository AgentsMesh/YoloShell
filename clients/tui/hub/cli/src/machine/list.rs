use crate::constants::*;
use crate::output;
use anyhow::Result;
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;
use yolo_hub_routing::{Right, Rights};

fn rights_text(rights: &Rights) -> String {
    [
        Right::Observe,
        Right::TerminalRead,
        Right::TerminalWrite,
        Right::Lifecycle,
        Right::AgentControl,
        Right::FileRead,
        Right::ConversationRead,
    ]
    .into_iter()
    .filter(|right| rights.allows(&[(*right)].into_iter().collect()))
    .map(|right| format!("{right:?}").to_lowercase())
    .collect::<Vec<_>>()
    .join(",")
}

pub(crate) async fn list(socket: &Path, entry: HubId, json: bool, deadline: Instant) -> Result<()> {
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
        let relation = if machine.consume {
            "observe-only"
        } else {
            "pinned"
        };
        let imports = machine
            .exports
            .iter()
            .map(|policy| format!("{} [{}]", policy.origin, rights_text(&policy.rights)))
            .collect::<Vec<_>>();
        let exports = imports.join(",");
        println!(
            "{}\t{}\t{}\t{}\t{}",
            label,
            machine.identity.hub(),
            socket,
            relation,
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
