use super::{
    federation_agents::launch,
    federation_support::{
        GRAPH_TIMEOUT, NAME_C, RunningNode, TEMP_ROOT, deadline, shell_relay_config,
    },
};
use crate::Harness;
use std::{path::Path, sync::Arc};
use yolo_hub_client::ClientError;
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget};
use yolo_hub_rpc_protocol::{Command, Method, Reply};
use yolo_hub_state::HubState;
use yolo_hub_terminal_protocol::{ATTACHMENT_CHUNK_BYTES, PublicError, STAGE_ATTACHMENT_METHOD};

const NAME: &str = "remote screenshot 图片.png";
const PATTERN_LENGTH: usize = 251;

#[tokio::test(flavor = "multi_thread")]
async fn real_three_hub_file_paste_publishes_to_the_exact_runtime_and_rejects_stale_epochs() {
    let a = crate::agent_recovery::federation_harness().await;
    let b = crate::agent_recovery::federation_harness().await;
    let c = crate::agent_recovery::federation_harness().await;
    let state = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let directory = state.path().canonicalize().unwrap();
    let entry = Arc::new(HubState::open(&directory.join("entry"), a.home.path()).unwrap());
    let relay = Arc::new(HubState::open(&directory.join("relay"), b.home.path()).unwrap());
    let origin = Arc::new(HubState::open(&directory.join("origin"), c.home.path()).unwrap());
    let (entry_policy, relay_policy, origin_policy) = shell_relay_config(&entry, &relay, &origin);
    let entry = RunningNode::start(&a, entry, entry_policy).await;
    let relay = RunningNode::start(&b, relay, relay_policy).await;
    let origin = RunningNode::start(&c, origin, origin_policy).await;
    let target = launch(&origin, &c, NAME_C).await;
    entry.wait_origins(&[target.address.hub_id]).await;
    verify(&entry, &c, target).await;
    entry.stop().await;
    relay.stop().await;
    origin.stop().await;
}

async fn verify(entry: &RunningNode, origin: &Harness, target: RuntimeTarget) {
    let data = (0..ATTACHMENT_CHUNK_BYTES * 2 + 1)
        .map(|offset| (offset % PATTERN_LENGTH) as u8)
        .collect::<Vec<_>>();
    let mut client = entry.client().await;
    assert!(!client.hello().methods.contains(&Method::StageAttachment));
    let reply = client
        .call(target.address.hub_id, Command::Extensions {}, deadline())
        .await
        .unwrap();
    let Reply::Extensions { methods } = reply else {
        panic!("missing extension catalogue")
    };
    assert!(
        methods
            .iter()
            .any(|method| method == STAGE_ATTACHMENT_METHOD)
    );
    let path = client
        .stage_attachment(target, NAME, &mut data.as_slice(), GRAPH_TIMEOUT)
        .await
        .unwrap();
    assert!(
        Path::new(&path).starts_with(
            origin
                .paths()
                .session_dir(target.address.terminal_id.as_uuid())
        )
    );
    assert_eq!(std::fs::read(path).unwrap(), data);
    let stale = RuntimeTarget {
        runtime_epoch: RuntimeEpoch::new(),
        ..target
    };
    let error = client
        .stage_attachment(stale, NAME, &mut &[][..], GRAPH_TIMEOUT)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Remote(PublicError::TargetMismatch)
    ));
}

pub async fn denied(entry: &RunningNode, origin: &Harness, target: RuntimeTarget) {
    let mut client = entry.client().await;
    let error = client
        .stage_attachment(target, NAME, &mut &[][..], GRAPH_TIMEOUT)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Remote(PublicError::Unreachable | PublicError::Unauthorized)
    ));
    let dir = yolo_paths::attachments::directory(
        &origin
            .paths()
            .session_dir(target.address.terminal_id.as_uuid()),
    );
    assert!(!dir.exists());
}
