use super::{RunningNode, capture::wait_screen};
use yolo_runtime::{model::WriterLeaseGrant, worker::RuntimeRef};
#[path = "federation_tui_inventory_identity.rs"]
mod identity;

pub async fn exercise(
    node: &RunningNode,
    client: &yolo_client::Client,
    target: RuntimeRef,
    grant: &WriterLeaseGrant,
    output: &mut yolo_client::Attachment,
    screen: &mut vt100::Parser,
    visible: &[&str],
) {
    let identities = identity::expected(node, visible).await;
    send(client, target, grant, SIDEBAR).await;
    let all_agents = format!("{AGENTS_HEADER}{}", visible.len());
    wait_screen(output, screen, |text| {
        text.contains(&all_agents) && text.contains(ALL_HUBS)
    })
    .await;
    // Compact rows omit internal IDs. Search by the exact origin-qualified
    // identity, then inspect its details without opening or acquiring its PTY.
    for identity in &identities {
        append_search(client, target, grant, output, screen, &identity.query()).await;
        let single_agent = format!("{AGENTS_HEADER}{SINGLE_ORIGIN_AGENTS}");
        wait_screen(output, screen, |text| text.contains(&single_agent)).await;
        send(client, target, grant, DETAILS).await;
        wait_screen(output, screen, |text| {
            identity.rendered(text) && text.contains(HOOK_SOURCE)
        })
        .await;
        send(client, target, grant, ESCAPE).await;
        send(client, target, grant, ESCAPE).await;
        wait_screen(output, screen, |text| text.contains(&all_agents)).await;
    }
    let directory = identities
        .first()
        .unwrap()
        .agent
        .fields()
        .working_directory
        .as_ref()
        .unwrap();
    append_search(client, target, grant, output, screen, directory.as_str()).await;
    let single_agent = format!("{AGENTS_HEADER}{SINGLE_ORIGIN_AGENTS}");
    wait_screen(output, screen, |text| {
        text.contains(&single_agent) && !text.contains(SEARCH_STATUS)
    })
    .await;
    append_search(
        client,
        target,
        grant,
        output,
        screen,
        &uuid::Uuid::new_v4().to_string(),
    )
    .await;
    let no_agents = format!("{AGENTS_HEADER}{NO_AGENTS}");
    wait_screen(output, screen, |text| {
        text.contains(&no_agents) && text.contains(EMPTY_AGENTS) && !text.contains(SEARCH_STATUS)
    })
    .await;
    send(client, target, grant, ESCAPE).await;
    wait_screen(output, screen, |text| text.contains(&all_agents)).await;
    send(client, target, grant, TAB).await;
    let all_hubs = format!("{HUBS_HEADER}{}", visible.len());
    wait_screen(output, screen, |text| {
        text.contains(&all_hubs) && text.contains(REACHABLE)
    })
    .await;
    send(client, target, grant, ENTER).await;
    let scoped = format!("{AGENTS_HEADER}{SINGLE_ORIGIN_AGENTS}");
    wait_screen(output, screen, |text| text.contains(&scoped)).await;
    send(client, target, grant, ESCAPE).await;
    wait_screen(output, screen, |text| {
        text.contains(&all_agents) && text.contains(ALL_HUBS)
    })
    .await;
    // The caller selects terminal names; agent run titles have a different contract.
    send(client, target, grant, TERMINAL_PICKER).await;
    wait_screen(output, screen, |text| {
        text.contains(TERMINALS_HEADER) && visible.iter().all(|name| text.contains(name))
    })
    .await;
}

async fn append_search(
    client: &yolo_client::Client,
    target: RuntimeRef,
    grant: &WriterLeaseGrant,
    output: &mut yolo_client::Attachment,
    screen: &mut vt100::Parser,
    query: &str,
) {
    send(client, target, grant, SEARCH).await;
    wait_screen(output, screen, |text| text.contains(SEARCH_STATUS)).await;
    let mut input = query.as_bytes().to_vec();
    input.extend_from_slice(ENTER);
    send(client, target, grant, &input).await;
}

async fn send(
    client: &yolo_client::Client,
    target: RuntimeRef,
    grant: &WriterLeaseGrant,
    bytes: &[u8],
) {
    client
        .input(
            target,
            grant.generation,
            &grant.writer_capability,
            bytes.to_vec(),
        )
        .await
        .unwrap();
}

const TAB: &[u8] = b"\t";
const SIDEBAR: &[u8] = b"\x02s";
const TERMINAL_PICKER: &[u8] = b"\x02g";
const DETAILS: &[u8] = b"I";
const ENTER: &[u8] = b"\r";
const ESCAPE: &[u8] = b"\x1b";
const SEARCH: &[u8] = b"/";
const SEARCH_STATUS: &str = "Filter:";
const AGENTS_HEADER: &str = "Agent runs: ";
const TERMINALS_HEADER: &str = "Terminals: ";
const EMPTY_AGENTS: &str = "No agent runs";
const HUBS_HEADER: &str = "Hubs: ";
const HOOK_SOURCE: &str = "Hooks";
const REACHABLE: &str = "Reachable";
const ALL_HUBS: &str = "All Hubs";
const SINGLE_ORIGIN_AGENTS: usize = 1;
const NO_AGENTS: usize = 0;
