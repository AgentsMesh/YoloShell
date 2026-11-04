use super::{
    super::{
        federation_support::{RunningNode, deadline},
        support::{acquire, input},
    },
    constants::*,
};
use crate::Harness;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::*;
use yolo_hub_terminal_client::Attachment;

pub async fn create(node: &RunningNode, harness: &Harness) -> RuntimeTarget {
    let token = uuid::Uuid::new_v4();
    let hub = node.node.hub_id();
    let request = CreateShellRequest::new(CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        creation_token: token,
        name: Some(SHELL_NAME.into()),
        cwd: harness.home.path().to_string_lossy().into_owned(),
        size: TerminalSize::default(),
    })
    .unwrap();
    let mut client = node.client().await;
    let Reply::Created { creation } = client
        .call(hub, Command::Create { request }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    let view = harness
        .client()
        .get_session(creation.address.terminal_id.as_uuid())
        .await
        .unwrap();
    harness.track_view(&view);
    assert!(view.agent.is_none());
    client
        .call(
            hub,
            Command::Finalize {
                creation_token: token,
                creation,
            },
            deadline(),
        )
        .await
        .unwrap();
    let Reply::Terminal { terminal: session } = client
        .call(
            hub,
            Command::Get {
                address: creation.address,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(session.fields().agent.is_none());
    session.fields().runtime.unwrap().target
}

pub async fn send(node: &RunningNode, target: RuntimeTarget, bytes: &[u8]) {
    let mut client = node.client().await;
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
    client
        .call(target.address.hub_id, input(grant, bytes), deadline())
        .await
        .unwrap();
    client
        .call(
            target.address.hub_id,
            Command::Release { grant },
            deadline(),
        )
        .await
        .unwrap();
}

pub async fn ping(node: &RunningNode, target: RuntimeTarget) {
    let nonce = uuid::Uuid::new_v4();
    let command =
        format!("printf '%s%s%s\\n' '{SHELL_MARKER_LEFT}' '{SHELL_MARKER_RIGHT}' '{nonce}'\r");
    let marker = format!("{SHELL_MARKER_LEFT}{SHELL_MARKER_RIGHT}{nonce}");
    let mut attachment = Attachment::open(
        node.client().await,
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    send(node, target, command.as_bytes()).await;
    let mut bytes = attachment
        .initial()
        .fields()
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.fields().data.clone())
        .unwrap_or_default();
    tokio::time::timeout_at(deadline(), async {
        while !String::from_utf8_lossy(&bytes).contains(&marker) {
            if let TerminalEvent::Output { chunk } =
                attachment.next_event(deadline()).await.unwrap()
            {
                bytes.extend_from_slice(&chunk.fields().data);
                assert!(bytes.len() <= super::super::support::MAX_OBSERVED_BYTES);
            }
        }
    })
    .await
    .unwrap();
}

pub async fn survives(
    harness: &Harness,
    target: RuntimeTarget,
    before: &yolo_runtime::model::TerminalView,
) {
    let after = harness
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    assert!(after.agent.is_none());
    assert_eq!(after.manifest.cwd, before.manifest.cwd);
    let expected = before.runtime.as_ref().unwrap();
    let actual = after.runtime.as_ref().unwrap();
    assert_eq!(actual.runtime_epoch, target.runtime_epoch.as_uuid());
    assert_eq!(actual.worker_id, expected.worker_id);
    assert_eq!(
        (actual.worker_pid, actual.worker_start_time),
        (expected.worker_pid, expected.worker_start_time)
    );
    assert_eq!(
        (actual.child_pid, actual.child_start_time),
        (expected.child_pid, expected.child_start_time)
    );
    assert!(
        crate::ProcessIdentity {
            pid: actual.child_pid,
            start_time: actual.child_start_time
        }
        .matches()
    );
    assert!(
        crate::ProcessIdentity {
            pid: actual.worker_pid,
            start_time: actual.worker_start_time
        }
        .matches()
    );
}
