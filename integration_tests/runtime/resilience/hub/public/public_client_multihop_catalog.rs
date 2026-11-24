use crate::hub::federation_support;
use std::collections::BTreeSet;
use yolo_hub_client::EndpointClient;
use yolo_hub_identity::{AgentAddress, HubId, RuntimeTarget, TerminalAddress};
use yolo_hub_rpc_protocol::{Command, Reply};

async fn hubs(client: &mut EndpointClient, entry_hub: HubId) -> BTreeSet<HubId> {
    let Reply::Hubs { hubs, next: None } = client
        .call(
            entry_hub,
            Command::ListHubs { after: None },
            federation_support::deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("public hub catalogue was not complete")
    };
    hubs.into_iter().map(|hub| hub.hub_id).collect()
}

async fn agents(client: &mut EndpointClient, entry_hub: HubId) -> BTreeSet<AgentAddress> {
    let Reply::Agents { agents, next: None } = client
        .call(
            entry_hub,
            Command::ListAgents { after: None },
            federation_support::deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("public agent catalogue was not complete")
    };
    agents
        .into_iter()
        .map(|agent| agent.cursor().target().address)
        .collect()
}

async fn terminals(client: &mut EndpointClient, entry_hub: HubId) -> BTreeSet<TerminalAddress> {
    let Reply::Terminals {
        terminals,
        next: None,
    } = client
        .call(
            entry_hub,
            Command::List {
                include_archived: false,
                after: None,
            },
            federation_support::deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("public session catalogue was not complete")
    };
    terminals
        .into_iter()
        .map(|session| session.fields().address)
        .collect()
}

pub async fn assert_views(
    a: &mut EndpointClient,
    b: &mut EndpointClient,
    c: &mut EndpointClient,
    a_hub: HubId,
    b_hub: HubId,
    c_hub: HubId,
    expected_agents: [AgentAddress; 3],
) {
    assert_eq!(hubs(a, a_hub).await, BTreeSet::from([a_hub, b_hub, c_hub]));
    assert_eq!(hubs(b, b_hub).await, BTreeSet::from([b_hub, c_hub]));
    assert_eq!(hubs(c, c_hub).await, BTreeSet::from([c_hub]));
    assert_eq!(agents(a, a_hub).await, BTreeSet::from(expected_agents));
    assert_eq!(
        agents(b, b_hub).await,
        BTreeSet::from([expected_agents[1], expected_agents[2]])
    );
    assert_eq!(agents(c, c_hub).await, BTreeSet::from([expected_agents[2]]));
}

pub async fn terminal_agent(
    node: &crate::hub::federation_support::RunningNode,
    target: RuntimeTarget,
) -> AgentAddress {
    node.agents()
        .await
        .into_iter()
        .find(|agent| agent.fields().terminal == target)
        .map(|agent| agent.cursor().target().address)
        .unwrap()
}

pub async fn assert_shell_visible(
    a: &mut EndpointClient,
    b: &mut EndpointClient,
    c: &mut EndpointClient,
    address: TerminalAddress,
    a_hub: HubId,
    b_hub: HubId,
    c_hub: HubId,
) {
    wait_session(a, a_hub, address).await;
    wait_session(b, b_hub, address).await;
    wait_session(c, c_hub, address).await;
}

async fn wait_session(client: &mut EndpointClient, hub: HubId, address: TerminalAddress) {
    tokio::time::timeout_at(federation_support::deadline(), async {
        loop {
            if terminals(client, hub).await.contains(&address) {
                return;
            }
            tokio::time::sleep(federation_support::POLL).await;
        }
    })
    .await
    .unwrap();
}
