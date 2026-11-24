use super::{federation_support::RunningNode, support::deadline};
use std::collections::BTreeSet;
use yolo_hub_client::Client;
use yolo_hub_rpc_protocol::{Command, Reply};

pub async fn verify(a: &RunningNode, b: &RunningNode, c: &RunningNode) {
    let a_hub = a.node.hub_id();
    let b_hub = b.node.hub_id();
    let c_hub = c.node.hub_id();
    for (node, origins) in [
        (a, BTreeSet::from([a_hub, b_hub, c_hub])),
        (b, BTreeSet::from([b_hub, c_hub])),
        (c, BTreeSet::from([c_hub])),
    ] {
        let mut client = Client::connect_local(&node.socket, node.node.hub_id(), deadline()).await.unwrap();
        assert_eq!(client.hello().hub_id, node.node.hub_id());
        let catalog = yolo_hub_catalog_client::read(&mut client, deadline()).await.unwrap();
        let agents = catalog.agents.unwrap();
        assert_eq!(agents.len(), origins.len());
        assert_eq!(
            agents
                .iter()
                .map(|agent| agent.cursor().target().address.hub_id)
                .collect::<BTreeSet<_>>(),
            origins
        );
        assert_eq!(
            catalog
                .terminals
                .iter()
                .map(|session| session.fields().address.hub_id)
                .collect::<BTreeSet<_>>(),
            origins
        );
        assert_eq!(
            catalog
                .hubs
                .unwrap()
                .iter()
                .map(|hub| hub.hub_id)
                .collect::<BTreeSet<_>>(),
            origins
        );
        for agent in agents {
            let target = agent.cursor().target();
            let reply = client
                .call(target.address.hub_id, Command::GetAgent { target }, deadline())
                .await
                .unwrap();
            let Reply::Agent { agent: queried } = reply else {
                panic!()
            };
            assert_eq!(queried.cursor().target(), agent.cursor().target());
        }
    }
}
