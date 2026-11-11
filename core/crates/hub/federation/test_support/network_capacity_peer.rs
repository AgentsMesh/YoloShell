use super::constants::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use yolo_hub_delegation::{
    DelegationChain, FIRST_POLICY_EPOCH, GrantClaims, PeerIdentity, SignedGrant,
};
use yolo_hub_peer_protocol::{
    CATALOGUE_SCHEMA_VERSION, Catalogue, ExportedOrigin, FIRST_REVISION,
    MAX_CATALOGUE_LIFETIME_MS, PeerReply, PeerRequest, Purpose, Signed,
};
use yolo_hub_peer_protocol_test_support::support::observe;
use yolo_hub_peer_rpc::{Handler, Response, RpcError, RpcFuture};
use yolo_hub_state::HubState;

pub struct DirectoryPeer {
    pub export: ExportedOrigin,
    pub invalid: AtomicBool,
    state: Arc<HubState>,
    revision: AtomicU64,
}

impl DirectoryPeer {
    pub fn new(state: Arc<HubState>, recipient: PeerIdentity) -> Self {
        let now = crate::node::now_ms().unwrap();
        let expires = now + MAX_CATALOGUE_LIFETIME_MS;
        let grant = SignedGrant::issue(
            GrantClaims {
                grant_id: uuid::Uuid::new_v4(),
                issuer: state.peer_identity(),
                recipient,
                origin: state.peer_identity(),
                policy_epoch: FIRST_POLICY_EPOCH,
                not_before_ms: now,
                expires_at_ms: expires,
                remaining_reexports: u8::default(),
                rights: observe(),
                parent: None,
            },
            state.as_ref(),
        )
        .unwrap();
        let catalogue = Signed::new(
            Catalogue {
                schema_version: CATALOGUE_SCHEMA_VERSION,
                revision: FIRST_REVISION,
                policy_epoch: FIRST_POLICY_EPOCH,
                issued_at_ms: now,
                expires_at_ms: expires,
                terminals: Vec::new(),
                agents: Vec::new(),
                runs: Vec::new(),
                workspaces: Vec::new(),
            },
            Purpose::Catalogue,
            state.as_ref(),
        )
        .unwrap();
        Self {
            export: ExportedOrigin {
                catalogue,
                authority: DelegationChain::new(vec![grant]).unwrap(),
                reachable: true,
            },
            invalid: AtomicBool::new(false),
            state,
            revision: AtomicU64::new(FIRST_REVISION + REVISION_STEP),
        }
    }
}

impl Handler for DirectoryPeer {
    fn request(&self, request: PeerRequest) -> RpcFuture<'_, Response> {
        Box::pin(async move {
            let PeerRequest::Discover { request_id, .. } = request else {
                return Err(RpcError::Invalid);
            };
            let revision = self.revision.fetch_add(REVISION_STEP, Ordering::AcqRel);
            let mut export = self.export.clone();
            let mut catalogue = export.catalogue.payload;
            catalogue.revision = revision;
            export.catalogue = Signed::new(catalogue, Purpose::Catalogue, self.state.as_ref())
                .map_err(|_| RpcError::Invalid)?;
            if self.invalid.load(Ordering::Acquire) {
                *export.catalogue.signature.first_mut().unwrap() ^= SIGNATURE_CORRUPTION_MASK;
            }
            Ok(Response {
                reply: PeerReply::Directory {
                    request_id,
                    revision,
                    origins: vec![export],
                },
                updates: None,
            })
        })
    }
}
