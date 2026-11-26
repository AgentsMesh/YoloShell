use super::{
    super::federation_support::{POLL, RunningNode, deadline},
    constants::*,
};
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::{AgentAddress, RuntimeTarget};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalRecoveryHint, TerminalSummary};

pub async fn agent(
    node: &RunningNode,
    address: AgentAddress,
    freshness: Freshness,
) -> AgentSnapshot {
    matching(node, address, freshness, None).await
}

pub async fn at(node: &RunningNode, cursor: ObservationCursor) -> AgentSnapshot {
    matching(
        node,
        cursor.target().address,
        Freshness::Current,
        Some(cursor),
    )
    .await
}

async fn matching(
    node: &RunningNode,
    address: AgentAddress,
    freshness: Freshness,
    expected: Option<ObservationCursor>,
) -> AgentSnapshot {
    tokio::time::timeout_at(deadline(), async {
        loop {
            if let Some(agent) = node
                .agents()
                .await
                .into_iter()
                .find(|item| item.cursor().target().address == address)
            {
                let expected_cursor = expected.is_none_or(|cursor| {
                    agent.cursor().target() == cursor.target()
                        && agent.cursor().stream() == cursor.stream()
                        && agent.cursor().sequence() >= cursor.sequence()
                });
                if agent.fields().freshness == freshness && expected_cursor {
                    assert_eq!(agent.fields().source, ObservationSource::Connector);
                    assert_eq!(agent.fields().capabilities, AgentCapabilities::UNSUPPORTED);
                    assert_eq!(agent.fields().recovery, RecoveryStatus::Unsupported);
                    assert!(agent.fields().interaction.is_none());
                    return agent;
                }
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}

pub async fn session(node: &RunningNode, target: RuntimeTarget) -> TerminalSummary {
    let Reply::Terminal { terminal: session } = node
        .client()
        .await
        .call(
            target.address.hub_id,
            Command::Get {
                address: target.address,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(session.fields().runtime.unwrap().target, target);
    assert_eq!(
        session.fields().runtime.unwrap().status,
        RuntimeStatus::Running
    );
    assert_eq!(session.fields().recovery, Some(TerminalRecoveryHint::Ready));
    session
}

pub async fn bound(node: &RunningNode, expected: &AgentSnapshot) {
    let session = session(node, expected.fields().terminal).await;
    assert_eq!(
        session.fields().agent,
        Some(expected.cursor().target())
    );
    let Reply::Agent { agent } = node
        .client()
        .await
        .call(
            expected.cursor().target().address.hub_id,
            Command::GetAgent {
                target: expected.cursor().target(),
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(agent.cursor(), expected.cursor());
    assert_eq!(agent.fields().terminal, expected.fields().terminal);
    assert_eq!(agent.fields().source, ObservationSource::Connector);
    assert_eq!(agent.fields().provider.kind(), PROVIDER_KIND);
    assert_eq!(
        agent.fields().working_directory,
        expected.fields().working_directory
    );
}

pub fn same(before: &AgentSnapshot, after: &AgentSnapshot) {
    assert_eq!(before.cursor().target(), after.cursor().target());
    assert_eq!(before.cursor().stream(), after.cursor().stream());
    assert_eq!(before.fields().terminal, after.fields().terminal);
}
