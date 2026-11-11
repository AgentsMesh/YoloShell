use super::super::constants::CODEX_ACK_PROCESS;
use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_live_source::PeerEvidence;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::{FIRST_EVENT_SEQUENCE, Fixture};

const LIVE_HOME: &str = "ingress-live-production-home";
const WRONG_LINEAGE_HOME: &str = "ingress-live-wrong-lineage-home";
const WRONG_GENERATION_HOME: &str = "ingress-live-wrong-generation-home";

#[tokio::test]
async fn sequence_zero_occurrence_traverses_the_production_chain_and_replays() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(LIVE_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let runtime = endpoint.start(&supervisor, &fixture).await;

    let accepted = endpoint
        .exchange_detailed(&socket, &fixture.start(FIRST_EVENT_SEQUENCE))
        .await;
    assert_eq!(
        accepted.occurrence.sequence,
        crate::agent::hooks::EventSequence::ZERO
    );
    assert_eq!(accepted.ack.sequence, Some(FIRST_EVENT_SEQUENCE));
    assert_eq!(
        runtime.state().unwrap().unwrap().last_sequence.0,
        FIRST_EVENT_SEQUENCE
    );

    let replay = endpoint.replay(&socket, &accepted).await;
    assert_eq!(replay.code, crate::codex_hooks::ACK_CODE_REPLAY);
    assert_eq!(
        runtime.state().unwrap().unwrap().last_sequence.0,
        FIRST_EVENT_SEQUENCE
    );
    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}

#[tokio::test]
async fn wrong_direct_parent_is_rejected_without_authority_mutation() {
    assert_peer_rejected(WRONG_LINEAGE_HOME, PeerEvidence::WrongLineage).await;
}

#[tokio::test]
async fn reused_peer_pid_generation_is_rejected_without_authority_mutation() {
    assert_peer_rejected(WRONG_GENERATION_HOME, PeerEvidence::WrongGeneration).await;
}

async fn assert_peer_rejected(home: &str, evidence: PeerEvidence) {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(home);
    let supervisor = CodexHookIngressSupervisor::new();
    let socket = endpoint.socket_path();
    let runtime = endpoint.start(&supervisor, &fixture).await;
    let before = runtime.state().unwrap().unwrap();

    let ack = endpoint
        .exchange_rejected(&socket, &fixture.start(FIRST_EVENT_SEQUENCE), evidence)
        .await;

    assert_eq!(ack.code, CODEX_ACK_PROCESS);
    assert_eq!(runtime.state().unwrap().unwrap(), before);
    supervisor.shutdown().await.unwrap();
    endpoint.finish();
}
