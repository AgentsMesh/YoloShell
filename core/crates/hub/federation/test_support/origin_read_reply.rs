use crate::Federation;
use std::sync::Arc;
use yolo_hub_delegation::{DelegationChain, GrantClaims, SignedGrant};
use yolo_hub_federation_settings::{Configuration, ExportPolicy, Peer};
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_peer_protocol::{Call, MAX_CALL_LIFETIME_MS, Purpose, Signed, required_rights};
use yolo_hub_peer_protocol_test_support::support::{Identity, observe};
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_state::HubState;

#[path = "constants.rs"]
mod constants;

pub struct OriginFixture {
    pub node: Arc<Federation>,
    pub peer: Identity,
    _root: tempfile::TempDir,
}

impl OriginFixture {
    pub async fn new(index: usize) -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().canonicalize().unwrap();
        let home = path.join(constants::HOME_DIRECTORY);
        let state =
            Arc::new(HubState::open(&path.join(constants::STATE_DIRECTORY), &home).unwrap());
        let backend =
            LocalBackend::new(state.hub_id(), &home, &home.join(constants::HOST_SOCKET)).unwrap();
        let peer = Identity::new();
        let command = super::resource_support::resource_reads(state.hub_id()).remove(index);
        let configuration = Configuration {
            peers: vec![Peer {
                identity: peer.identity,
                dial: None,
                consume: false,
                exports: vec![ExportPolicy {
                    origin: state.hub_id(),
                    rights: required_rights(&command)
                        .expect("resource read rights")
                        .union(&observe()),
                    remaining_reexports: u8::default(),
                }],
            }],
            ..Configuration::default()
        };
        let node = Federation::new(state, backend, configuration)
            .await
            .unwrap();
        Self {
            node,
            peer,
            _root: root,
        }
    }

    pub fn call(&self, command: Command, expired: bool) -> Signed<Call> {
        let now = crate::node::now_ms().unwrap();
        let grant = SignedGrant::issue(
            GrantClaims {
                grant_id: uuid::Uuid::new_v4(),
                issuer: self.node.state.peer_identity(),
                recipient: self.peer.identity,
                origin: self.node.state.peer_identity(),
                policy_epoch: self.node.policy_epoch,
                not_before_ms: now,
                expires_at_ms: now + MAX_CALL_LIFETIME_MS,
                remaining_reexports: u8::default(),
                rights: required_rights(&command).expect("resource read rights"),
                parent: None,
            },
            self.node.state.as_ref(),
        )
        .unwrap();
        Signed::new(
            Call {
                request: Request::new(self.node.hub_id(), command).unwrap(),
                authority: DelegationChain::new(vec![grant]).unwrap(),
                issued_at_ms: if expired {
                    now.saturating_sub(MAX_CALL_LIFETIME_MS)
                } else {
                    now
                },
                expires_at_ms: if expired {
                    now
                } else {
                    now + MAX_CALL_LIFETIME_MS
                },
            },
            Purpose::Call,
            &self.peer,
        )
        .unwrap()
    }
}
