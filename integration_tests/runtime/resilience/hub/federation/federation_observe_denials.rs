use super::super::{
    federation_agents::{CreatedAgent, ping},
    federation_creation_query as creation_query,
    federation_support::*,
    support::{MAX_OBSERVED_BYTES, acquire, input},
};
use crate::Harness;
use yolo_hub_client::ClientError;
use yolo_hub_identity::AgentAddress;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    AttachRequest, LeaseMode, PublicError, TerminalAction, TerminalCommand, TerminalSelection,
    TerminalEvent,
};
use yolo_hub_terminal_client::Attachment;

pub async fn verify(
    a: &RunningNode,
    b: &RunningNode,
    c: &RunningNode,
    c_host: &Harness,
    created: &CreatedAgent,
    address: AgentAddress,
) {
    let target = created.target;
    creation_query::verify(&[c], created).await;
    let before = c_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let mut origin = c.client().await;
    let Reply::Lease { grant } = origin
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
    let mut output = Attachment::open(
        c.client().await,
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    let Reply::Agent { agent } = c
        .client()
        .await
        .call(
            target.address.hub_id,
            Command::GetCurrentAgent {
                address,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let agent_target = agent.cursor().target();
    let (rejected_input, rejected_marker) = ping();
    for entry in [a, b] {
        let mut client = entry.client().await;
        let Reply::Agent { agent } = client
            .call(
                target.address.hub_id,
                Command::GetAgent {
                    target: agent_target,
                },
                deadline(),
            )
            .await
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(agent.cursor().target(), agent_target);
        let commands = [
            creation_query::command(created),
            acquire(target, LeaseMode::Acquire {}),
            input(grant, &rejected_input),
            Command::Attach {
                request: AttachRequest::new(target, None).unwrap(),
            },
            Command::Terminal {
                command: TerminalCommand {
                    request_id: uuid::Uuid::new_v4(),
                    selection: TerminalSelection {
                        address: target.address,
                        expected_runtime: Some(target.runtime_epoch),
                    },
                    action: TerminalAction::Terminate { force: false },
                },
            },
        ];
        for command in commands {
            let error = client
                .call(target.address.hub_id, command, deadline())
                .await
                .unwrap_err();
            assert!(
                matches!(error, ClientError::Remote(PublicError::Unreachable)),
                "unexpected denial: {error}"
            );
            assert!(!error.outcome_is_unknown());
            assert!(client.is_usable());
        }
    }
    // A valid origin-side capability alone never expands delegated peer rights.
    let (authorized_input, authorized_marker) = ping();
    origin
        .call(
            target.address.hub_id,
            input(grant, &authorized_input),
            deadline(),
        )
        .await
        .unwrap();
    let mut observed = output
        .initial()
        .fields()
        .snapshot
        .as_ref()
        .unwrap()
        .fields()
        .data
        .clone();
    tokio::time::timeout_at(deadline(), async {
        while !String::from_utf8_lossy(&observed).contains(&authorized_marker) {
            if let TerminalEvent::Output { chunk } = output.next_event(deadline()).await.unwrap() {
                observed.extend_from_slice(&chunk.fields().data);
                assert!(observed.len() <= MAX_OBSERVED_BYTES);
            }
        }
    })
    .await
    .unwrap();
    assert!(!String::from_utf8_lossy(&observed).contains(&rejected_marker));
    let after = c_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    assert!(!after.manifest.archived);
    assert_eq!(
        after.runtime.as_ref().unwrap().runtime_epoch,
        target.runtime_epoch.as_uuid()
    );
    assert_eq!(
        after.agent.as_ref().unwrap().process_pid,
        before.agent.as_ref().unwrap().process_pid
    );
    assert_eq!(
        after.agent.as_ref().unwrap().process_start_time,
        before.agent.as_ref().unwrap().process_start_time
    );
    origin
        .call(
            target.address.hub_id,
            Command::Release { grant },
            deadline(),
        )
        .await
        .unwrap();
    drop(output);
}
