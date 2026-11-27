use crate::{
    client::ClientCommand,
    constants::{CLIENT_HEADER, CLIENT_KIND_CLIENT, CLIENT_KIND_PRIMARY, NO_LOCAL_HUB},
    selection::Selection,
};
use anyhow::{Context, Result, ensure};
use yolo_hub_identity::HubId;
use yolo_local_profile::LocalProfile;
use yolo_tui_tab::ClientDirectory;

const NO_CLIENT: &str = "No attached terminal client was found";
const MANY_CLIENTS: &str =
    "Several terminal clients are attached; use yolo client list, then yolo client detach <CLIENT_ID>";
const DETACHED: &str = "Detached terminal client; shells and agents are still running";

/// One attached client as `yolo client list` reports it. The Hub-bound client
/// directory is addressed by the Hub id, so its kind makes that visible
/// instead of presenting a Hub id as an ordinary client id.
pub(crate) struct ClientRow {
    pub(crate) id: uuid::Uuid,
    pub(crate) kind: &'static str,
}

pub(crate) async fn run(selection: Selection, command: ClientCommand) -> Result<()> {
    let hub = identity(selection)?;
    let mut active = Vec::new();
    let mut handles = std::collections::BTreeMap::new();
    for client in yolo_tui_tab::client_directories(hub)? {
        if let Some(handle) = yolo_tui_client_control::inspect(&client.path).await? {
            handles.insert(client.id, handle);
            active.push(client);
        }
    }
    match command {
        ClientCommand::List { json } => {
            let rows = client_rows(&active, hub);
            print!("{}", clients_output(&rows, json));
        }
        ClientCommand::Detach { client } => {
            let selected = choose(&active, client)?;
            handles
                .remove(&selected.id)
                .context(NO_CLIENT)?
                .detach()
                .await?;
            println!("{DETACHED}");
        }
        _ => unreachable!("only client commands use local control discovery"),
    }
    Ok(())
}

pub(crate) fn client_rows(active: &[ClientDirectory], hub: HubId) -> Vec<ClientRow> {
    let mut rows: Vec<ClientRow> = active
        .iter()
        .map(|client| ClientRow {
            id: client.id,
            kind: if client.id == hub.as_uuid() {
                CLIENT_KIND_PRIMARY
            } else {
                CLIENT_KIND_CLIENT
            },
        })
        .collect();
    rows.sort_by_key(|row| (row.kind != CLIENT_KIND_PRIMARY, row.id));
    rows
}

pub(crate) fn clients_output(rows: &[ClientRow], json: bool) -> String {
    if json {
        let clients: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| serde_json::json!({ "id": row.id, "kind": row.kind }))
            .collect();
        let mut output = serde_json::to_string_pretty(&clients).expect("client rows serialize");
        output.push('\n');
        return output;
    }
    let mut output = format!("{CLIENT_HEADER}\n");
    for row in rows {
        output.push_str(&format!("{}\t{}\n", row.id, row.kind));
    }
    output
}

fn identity(selection: Selection) -> Result<HubId> {
    match selection {
        Selection::Explicit(options) => Ok(options.hub),
        Selection::Local { profile, .. } => {
            let profile = match profile {
                Some(path) => LocalProfile::load(&path)?,
                None => yolo_local_profile::current()?,
            };
            yolo_hub_state::HubState::inspect_identity(&profile.state, &profile.host_home)?
                .context(NO_LOCAL_HUB)
        }
    }
}

pub(crate) fn choose(
    clients: &[ClientDirectory],
    selected: Option<uuid::Uuid>,
) -> Result<&ClientDirectory> {
    if let Some(id) = selected {
        return clients
            .iter()
            .find(|client| client.id == id)
            .context(NO_CLIENT);
    }
    ensure!(clients.len() <= 1, MANY_CLIENTS);
    clients.first().context(NO_CLIENT)
}
