use super::federation_support::{RunningNode, deadline};
use std::collections::BTreeSet;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};

pub async fn verify(a: &RunningNode, b: &RunningNode, c: &RunningNode) {
    let (a_id, b_id, c_id) = (a.node.hub_id(), b.node.hub_id(), c.node.hub_id());
    assert_eq!(query(a, a_id).await, BTreeSet::from([a_id, b_id, c_id]));
    assert_eq!(query(b, b_id).await, BTreeSet::from([b_id, c_id]));
    assert_eq!(query(c, c_id).await, BTreeSet::from([c_id]));
    // Origin requests never expose that origin's imported directory.
    assert_eq!(query(a, b_id).await, BTreeSet::from([b_id]));
}

async fn query(entry: &RunningNode, destination: HubId) -> BTreeSet<HubId> {
    let mut client = entry.client().await;
    let Reply::Hubs { hubs, next: None } = client
        .call(destination, Command::ListHubs { after: None }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(hubs.iter().all(|hub| hub.reachable));
    hubs.into_iter().map(|hub| hub.hub_id).collect()
}
