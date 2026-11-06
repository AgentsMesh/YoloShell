use super::{super::federation_support::*, constants::*, node::ConnectorNode};
use crate::Harness;
use std::{path::Path, sync::Arc};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_state::HubState;

pub struct Graph {
    pub a: RunningNode,
    pub b: RunningNode,
    pub c: ConnectorNode,
}
impl Graph {
    pub async fn start(root: &Path, a_host: &Harness, b_host: &Harness, c_host: &Harness) -> Self {
        let a_state = Arc::new(HubState::open(&root.join(A_STATE), a_host.home.path()).unwrap());
        let b_state = Arc::new(HubState::open(&root.join(B_STATE), b_host.home.path()).unwrap());
        let c_state = Arc::new(HubState::open(&root.join(C_STATE), c_host.home.path()).unwrap());
        let (a, b, c) = (
            a_state.peer_identity(),
            b_state.peer_identity(),
            c_state.peer_identity(),
        );
        let a_config = Configuration {
            trusted_origins: vec![c],
            file_roots: vec![],
            peers: vec![peer(b, Some(&b_state), true, vec![])],
        };
        let b_config = Configuration {
            trusted_origins: vec![],
            file_roots: vec![],
            peers: vec![
                peer(
                    a,
                    None,
                    false,
                    vec![exports(b.hub(), false), exports(c.hub(), false)],
                ),
                peer(c, Some(&c_state), true, vec![]),
            ],
        };
        let c_config = Configuration {
            trusted_origins: vec![],
            file_roots: vec![],
            peers: vec![peer(b, Some(&b_state), false, vec![exports(c.hub(), true)])],
        };
        let a = RunningNode::start(a_host, a_state, a_config).await;
        let b = RunningNode::start(b_host, b_state, b_config).await;
        let c = ConnectorNode::start(c_host, c_state, c_config).await;
        Self { a, b, c }
    }
    pub async fn stop(self) {
        self.a.stop().await;
        self.b.stop().await;
        self.c.stop().await;
    }
}

pub async fn standalone(harness: &Harness, path: &Path) -> ConnectorNode {
    let state = Arc::new(HubState::open(path, harness.home.path()).unwrap());
    let config = Configuration {
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![],
    };
    ConnectorNode::start(harness, state, config).await
}
