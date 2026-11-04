use super::gate::DropGate;
use crate::Federation;
use std::sync::Arc;
use tokio::sync::Notify;
use yolo_hub_delegation::{DelegationChain, GrantClaims, SignedGrant};
use yolo_hub_dispatch::{EventFuture, TerminalSource};
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_peer_protocol::{Call, MAX_CALL_LIFETIME_MS, Purpose, Signed, required_rights};
use yolo_hub_peer_protocol_test_support::support::Identity;
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_terminal_protocol::{
    AttachRequest, AttachResult, AttachResultFields, FIRST_OUTPUT_SEQUENCE, OutputCursor,
};

pub struct DelayedSource {
    pub _gate: DropGate,
    pub initial: AttachResult,
    pub entered: Arc<Notify>,
}
impl TerminalSource for DelayedSource {
    fn initial(&self) -> &AttachResult {
        &self.initial
    }
    fn next_event(&mut self) -> EventFuture<'_> {
        Box::pin(async {
            self.entered.notify_one();
            std::future::pending().await
        })
    }
}

pub fn call(node: &Federation, peer: &Identity) -> (Signed<Call>, AttachResult) {
    let now = crate::node::now_ms().unwrap();
    let target = RuntimeTarget {
        address: TerminalAddress {
            hub_id: node.hub_id(),
            terminal_id: TerminalId::new(),
        },
        runtime_epoch: RuntimeEpoch::new(),
    };
    let request = AttachRequest::new(
        target,
        Some(OutputCursor {
            target,
            after_sequence: u64::default(),
        }),
    )
    .unwrap();
    let command = Command::Attach { request };
    let rights = required_rights(&command).expect("remote attach rights");
    let grant = SignedGrant::issue(
        GrantClaims {
            grant_id: uuid::Uuid::new_v4(),
            issuer: node.state.peer_identity(),
            recipient: peer.identity,
            origin: node.state.peer_identity(),
            policy_epoch: node.policy_epoch,
            not_before_ms: now,
            expires_at_ms: now + MAX_CALL_LIFETIME_MS,
            remaining_reexports: u8::default(),
            rights: rights.clone(),
            parent: None,
        },
        node.state.as_ref(),
    )
    .unwrap();
    let call = Signed::new(
        Call {
            request: Request::new(node.hub_id(), command).unwrap(),
            authority: DelegationChain::new(vec![grant]).unwrap(),
            issued_at_ms: now,
            expires_at_ms: now + MAX_CALL_LIFETIME_MS,
        },
        Purpose::Call,
        peer,
    )
    .unwrap();
    call.payload
        .authority
        .authorize(
            node.state.peer_identity(),
            node.policy_epoch,
            call.signer,
            now,
            &rights,
        )
        .unwrap();
    let initial = AttachResult::new(AttachResultFields {
        target,
        snapshot: None,
        chunks: Vec::new(),
        history: None,
        next_sequence: FIRST_OUTPUT_SEQUENCE,
        resynchronized: false,
    })
    .unwrap();
    (call, initial)
}
