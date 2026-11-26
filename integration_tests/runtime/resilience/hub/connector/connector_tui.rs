use super::graph::Graph;
use crate::hub::federation_support::{RunningNode, deadline};
use crate::{Harness, runtime_ref_from_view};
use yolo_hub_identity::{AgentAddress, AgentId, HubId};
use yolo_runtime::{model::WriterLeaseGrant, worker::RuntimeRef};
#[path = "../federation/federation_tui_capture.rs"]
mod capture;
#[path = "connector_tui_constants.rs"]
mod constants;
#[path = "../federation/federation_tui_lease.rs"]
mod lease;
use constants::*;

pub async fn exercise(graph: &Graph, agent: AgentAddress) {
    // The driver Host is outside the graph; its PTY must not inflate the visible Shell count.
    let harness = Harness::start().await;
    let origins = [
        graph.a.node.hub_id(),
        graph.b.node.hub_id(),
        graph.c.running().node.hub_id(),
    ];
    for (index, node) in [&graph.a, &graph.b, graph.c.running()]
        .into_iter()
        .enumerate()
    {
        inventory(node, &harness, agent, &origins[index..], &origins[..index]).await;
    }
}

async fn inventory(
    node: &RunningNode,
    harness: &Harness,
    agent: AgentAddress,
    visible: &[HubId],
    hidden: &[HubId],
) {
    let binary = crate::bazel_runfile(&std::env::var(BINARY_ENV).unwrap()).unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let driver = harness.create_cat(DRIVER_NAME).await;
    let target = runtime_ref_from_view(&driver);
    let client = harness.client();
    let grant = client
        .acquire_writer(
            target,
            DRIVER_NAME,
            Some(yolo_runtime::lease::constants::MAX_TTL_MS),
        )
        .await
        .unwrap();
    let renewal = lease::Renewal::start(client.clone(), target, grant.clone());
    client
        .resize(
            target,
            grant.generation,
            &grant.writer_capability,
            COLUMNS,
            ROWS,
        )
        .await
        .unwrap();
    let mut output = client
        .attach(
            driver.manifest.terminal_id,
            yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE,
        )
        .await
        .unwrap();
    let snapshot = output.initial.snapshot.as_ref().unwrap();
    let mut screen = vt100::Parser::new(ROWS, COLUMNS, SCROLLBACK);
    screen.process(&snapshot.data);
    let (launch, complete) =
        capture::launch(&binary, node, &workspace.path().canonicalize().unwrap());
    send(&client, target, &grant, launch.as_bytes()).await;
    capture::wait_screen(&mut output, &mut screen, |text| {
        count(text, TERMINALS_LABEL, visible.len())
            && visible.iter().all(|hub| text.contains(&hub.to_string()))
    })
    .await;
    for origin in hidden {
        assert!(!screen.screen().contents().contains(&origin.to_string()));
    }
    send(&client, target, &grant, SIDEBAR).await;
    capture::wait_screen(&mut output, &mut screen, |text| {
        count(text, AGENTS_LABEL, SINGLE_AGENT)
    })
    .await;
    send(&client, target, &grant, DETAILS).await;
    let agent_label = agent.to_string();
    capture::wait_screen(&mut output, &mut screen, |text| {
        count(text, AGENTS_LABEL, SINGLE_AGENT)
            && text.contains(CONNECTOR_STATUS)
            && text.contains(RESUME_UNSUPPORTED)
            && text.contains(&agent_label)
    })
    .await;
    assert!(!screen.screen().contents().contains(NATIVE_SOURCE));
    send(&client, target, &grant, ESCAPE).await;
    let unknown = AgentAddress {
        hub_id: agent.hub_id,
        agent_id: AgentId::new(),
    };
    send(&client, target, &grant, format!("/{unknown}\r").as_bytes()).await;
    capture::wait_screen(&mut output, &mut screen, |text| {
        count(text, AGENTS_LABEL, NO_AGENTS) && text.contains(NO_AGENTS_LABEL)
    })
    .await;
    send(&client, target, &grant, ESCAPE).await;
    capture::wait_screen(&mut output, &mut screen, |text| {
        count(text, AGENTS_LABEL, SINGLE_AGENT)
    })
    .await;
    send(&client, target, &grant, TAB).await;
    capture::wait_screen(&mut output, &mut screen, |text| {
        count(text, HUBS_LABEL, visible.len())
    })
    .await;
    // Sidebar IDs may be clipped; exact origin searches still prove routing membership.
    for (origin, count_expected) in visible
        .iter()
        .map(|hub| (hub, SINGLE_HUB))
        .chain(hidden.iter().map(|hub| (hub, NO_HUBS)))
    {
        send(&client, target, &grant, format!("/{origin}\r").as_bytes()).await;
        capture::wait_screen(&mut output, &mut screen, |text| {
            count(text, HUBS_LABEL, count_expected)
        })
        .await;
        send(&client, target, &grant, ESCAPE).await;
        capture::wait_screen(&mut output, &mut screen, |text| {
            count(text, HUBS_LABEL, visible.len())
        })
        .await;
    }
    send(&client, target, &grant, QUIT).await;
    capture::wait_screen(&mut output, &mut screen, |text| text.contains(&complete)).await;
    let grant = renewal.stop().await;
    client.release_writer(&grant, target).await.unwrap();
    output.detach().await.unwrap();
    client
        .terminate(driver.manifest.terminal_id, false)
        .await
        .unwrap();
}

fn count(text: &str, label: &str, expected: usize) -> bool {
    let prefix = format!("{label}: ");
    text.lines()
        .find_map(|header| header.split_once(&prefix))
        .and_then(|(_, remaining)| remaining.split_whitespace().next())
        == Some(expected.to_string().as_str())
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
