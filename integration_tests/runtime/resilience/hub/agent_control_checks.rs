use super::super::{
    federation_agents::ping,
    federation_support::{RunningNode, deadline},
    multipath::io,
    support::acquire,
};
use super::{
    canary,
    custody::{self, Baseline},
    policy::Delegation,
    requests,
};
use yolo_hub_agent_protocol::{
    AgentAction, Capability, FailureCode, Freshness, InteractionReference, InteractionResponse,
    ObservationSource, Reachability,
};
use yolo_hub_client::ClientError;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseMode, PublicError};

#[path = "agent_control_convergence.rs"]
mod convergence;
#[path = "agent_control_preconditions.rs"]
mod preconditions;

pub async fn exercise(
    entry: &RunningNode,
    relay: &RunningNode,
    origin: &RunningNode,
    harness: &crate::Harness,
    target: RuntimeTarget,
    mode: Delegation,
) {
    // Finish a real fixture response before taking the no-effect baseline.
    custody::settle(origin, target).await;
    let agents = origin.agents().await;
    let [agent] = agents.as_slice() else { panic!() };
    assert_eq!(agent.fields().terminal, target);
    assert_eq!(agent.fields().source, ObservationSource::ProviderHook);
    assert_eq!(agent.fields().freshness, Freshness::Current);
    assert_eq!(agent.fields().reachability, Reachability::Reachable);
    assert_eq!(
        agent.fields().capabilities.respond_interaction,
        Capability::Unsupported {}
    );
    for node in [entry, relay] {
        convergence::exact(node, agent).await;
    }
    let writer = match mode {
        Delegation::Control => entry,
        Delegation::Attenuated => relay,
    };
    let mut client = writer.client().await;
    let Reply::Lease { grant } = client
        .call(
            target.address.hub_id,
            acquire(target, LeaseMode::Acquire {}),
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(grant.fields().target, target);
    let mut output = io::attach(origin, target, None).await;
    let baseline = Baseline::capture(origin, harness, grant).await;
    let mut forbidden = None;
    match mode {
        Delegation::Control => {
            // If an implementation wrongly types a semantic response into the PTY,
            // the real fixture emits this unique marker and changes the snapshot.
            let (text, marker) = ping();
            forbidden = Some(marker);
            let response = AgentAction::Respond {
                interaction: InteractionReference {
                    target: agent.cursor().target(),
                    id: uuid::Uuid::new_v4(),
                },
                response: InteractionResponse::Text {
                    text: String::from_utf8(text).unwrap(),
                },
            };
            requests::receipt(
                &mut client,
                requests::request(agent.cursor(), response),
                grant,
                FailureCode::Unsupported,
            )
            .await;
            preconditions::verify(&mut client, agent.cursor(), grant, entry.node.hub_id()).await;
        }
        Delegation::Attenuated => {
            for node in [entry, relay] {
                requests::denied(node, agent.cursor(), grant).await;
            }
            let mut observer = entry.client().await;
            let error = observer
                .call(
                    target.address.hub_id,
                    acquire(target, LeaseMode::Acquire {}),
                    deadline(),
                )
                .await
                .unwrap_err();
            assert!(matches!(
                error,
                ClientError::Remote(PublicError::Unreachable)
            ));
            assert!(!error.outcome_is_unknown());
            assert!(observer.is_usable());
        }
    }
    baseline.unchanged(origin, harness, target).await;
    canary::confirm(&mut client, grant, &mut output, forbidden.as_deref()).await;
    baseline.after_canary(origin, harness, target).await;
    for node in [entry, relay, origin] {
        convergence::exact(node, agent).await;
    }
    output.detach(deadline()).await.unwrap();
    client
        .call(
            target.address.hub_id,
            Command::Release { grant },
            deadline(),
        )
        .await
        .unwrap();
}
