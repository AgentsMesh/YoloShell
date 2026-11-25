use support::*;
use yolo_hub_delegation::*;
pub use yolo_hub_delegation_test_support as support;
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_peer_protocol::*;
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_terminal_protocol::*;

pub const SESSION_NAME: &str = "peer session";
pub const DIRECTORY: &str = "/workspace";
pub mod agent_control;
pub mod directory;

pub struct Fixture {
    pub origin: Identity,
    pub relay: Identity,
    pub entry: Identity,
    pub export: ExportedOrigin,
}
impl Fixture {
    pub fn new() -> Self {
        let (origin, relay, entry) = (Identity::new(), Identity::new(), Identity::new());
        let root = SignedGrant::issue(root_claims(&origin, &relay), &origin).unwrap();
        let child = SignedGrant::issue(child_claims(&root, &entry), &relay).unwrap();
        let authority = DelegationChain::new(vec![root, child]).unwrap();
        let catalogue = Catalogue {
            schema_version: CATALOGUE_SCHEMA_VERSION,
            revision: FIRST_REVISION,
            policy_epoch: FIRST_POLICY_EPOCH,
            issued_at_ms: BEFORE,
            expires_at_ms: EXPIRES,
            terminals: vec![session(origin.identity)],
            agents: Vec::new(),
            runs: Vec::new(),
            workspaces: Vec::new(),
        };
        let export = ExportedOrigin {
            catalogue: Signed::new(catalogue, Purpose::Catalogue, &origin).unwrap(),
            authority,
            reachable: true,
        };
        Self {
            origin,
            relay,
            entry,
            export,
        }
    }
    pub fn call(&self) -> Signed<Call> {
        Signed::new(
            Call {
                request: Request::new(
                    self.origin.identity.hub(),
                    Command::Get {
                        address: self.export.catalogue.payload.terminals[0].fields().address,
                    },
                )
                .unwrap(),
                authority: self.export.authority.clone(),
                issued_at_ms: NOW,
                expires_at_ms: EXPIRES,
            },
            Purpose::Call,
            &self.entry,
        )
        .unwrap()
    }
}
pub fn session(origin: PeerIdentity) -> TerminalSummary {
    let address = TerminalAddress {
        hub_id: origin.hub(),
        terminal_id: TerminalId::new(),
    };
    TerminalSummary::new(TerminalSummaryFields {
        presentation: Default::default(),
        address,
        name: SESSION_NAME.into(),
        cwd: DIRECTORY.into(),
        archived: false,
        recovery: None,
        runtime: Some(RuntimeSummary {
            target: RuntimeTarget {
                address,
                runtime_epoch: RuntimeEpoch::new(),
            },
            status: RuntimeStatus::Running,
            output_sequence: 0,
            size: Some(TerminalSize::default()),
            safe_termination: None,
        }),
        agent: None,
        workspace: None,
    })
    .unwrap()
}
