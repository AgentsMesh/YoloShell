use super::constants::*;
use crate::hub::{
    federation_support::{POLL, deadline},
    relay_crash::process::RelayProcess,
    support::acquire,
};
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, Lifecycle, Reachability};
use yolo_hub_client::ClientError;
use yolo_hub_identity::{HubId, RuntimeTarget};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    AttachRequest, LeaseMode, PublicError, RuntimeStatus, TerminalRecoveryHint,
};

pub async fn agents(entry: &RelayProcess, hub: HubId, offline: bool) -> Vec<AgentSnapshot> {
    tokio::time::timeout_at(deadline(), async {
        loop {
            let Reply::Agents { agents, next: None } = entry
                .client()
                .await
                .call(hub, Command::ListAgents { after: None }, deadline())
                .await
                .unwrap()
            else {
                panic!()
            };
            if agents.len() == EXPECTED_AGENTS
                && agents.iter().all(|agent| {
                    let remote = agent.cursor().target().address.hub_id != hub;
                    let unreachable = offline && remote;
                    agent.fields().reachability
                        == if unreachable {
                            Reachability::Unreachable
                        } else {
                            Reachability::Reachable
                        }
                        && agent.fields().freshness
                            == if unreachable {
                                Freshness::Stale
                            } else {
                                Freshness::Current
                            }
                })
            {
                assert!(
                    agents
                        .iter()
                        .all(|agent| !matches!(agent.fields().lifecycle, Lifecycle::Exited { .. }))
                );
                return agents;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}

pub async fn offline(entry: &RelayProcess, hub: HubId, targets: &[RuntimeTarget]) {
    let mut client = entry.client().await;
    let Reply::Hubs { hubs, next: None } = client
        .call(hub, Command::ListHubs { after: None }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(hubs.len(), EXPECTED_AGENTS);
    assert!(
        hubs.iter()
            .all(|item| item.reachable == (item.hub_id == hub))
    );
    let Reply::Terminals {
        terminals,
        next: None,
    } = client
        .call(
            hub,
            Command::List {
                include_archived: false,
                after: None,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let remote = terminals
        .iter()
        .filter(|session| session.fields().address.hub_id != hub)
        .collect::<Vec<_>>();
    assert_eq!(remote.len(), EXPECTED_REMOTE_AGENTS);
    for session in remote {
        assert_eq!(
            session.fields().recovery,
            Some(TerminalRecoveryHint::Unavailable)
        );
        let runtime = session.fields().runtime.as_ref().unwrap();
        assert_eq!(runtime.status, RuntimeStatus::Unreachable);
        assert!(runtime.safe_termination.is_none());
        assert!(targets.contains(&runtime.target));
    }
    for target in targets {
        for command in [
            acquire(*target, LeaseMode::Acquire {}),
            Command::Attach {
                request: AttachRequest::new(*target, None).unwrap(),
            },
        ] {
            let result = client
                .call(target.address.hub_id, command, deadline())
                .await;
            assert!(
                matches!(result, Err(ClientError::Remote(PublicError::Unreachable))),
                "{REJECTION_DETAIL}"
            );
        }
    }
}
