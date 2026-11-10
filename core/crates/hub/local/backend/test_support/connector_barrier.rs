use super::cleanup_rpc::{ProtocolFixture, bounded};
use crate::LocalBackend;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Waker},
};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::PublicError;
use yolo_runtime::protocol::RequestEnvelope;

#[derive(Clone, Copy)]
pub enum Observation {
    Agents,
    Terminals,
    Terminal,
}

impl Observation {
    pub async fn read(
        self,
        backend: &LocalBackend,
        address: TerminalAddress,
    ) -> Result<(), PublicError> {
        match self {
            Self::Agents => backend.observe_agents().await.map(drop),
            Self::Terminals => backend.terminals(false).await.map(drop),
            Self::Terminal => backend.session(address).await.map(drop),
        }
    }
}

pub async fn accept_while<F: Future>(
    fixture: &ProtocolFixture,
    pending: &mut Pin<Box<F>>,
) -> (yolo_runtime_rpc_testing::Peer, RequestEnvelope) {
    bounded(async {
        tokio::select! {
            response = fixture.accept_request() => response,
            _ = pending.as_mut() => panic!("{EARLY_COMPLETION}"),
        }
    })
    .await
}

pub fn assert_pending<F: Future>(pending: Pin<&mut F>) {
    let mut context = Context::from_waker(Waker::noop());
    assert!(
        pending.poll(&mut context).is_pending(),
        "{EARLY_COMPLETION}"
    );
}

const EARLY_COMPLETION: &str = "operation completed before the controlled Host response";
