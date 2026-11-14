use super::{
    federation_agents::launch_managed, federation_support::*, file_root_ui_constants::*,
    management_support::ManagedNode, management_tui_driver::*,
};
use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_file_protocol::{
    FILE_PROTOCOL_SCHEMA_VERSION, ReadFileFields, ReadFileRequest, RelativeFilePath,
};
use yolo_hub_state::HubState;

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_edits_file_roots_with_preflight_conflicts_and_exact_path_preservation() {
    const STATE: &str = "entry";
    const DYNAMIC_PORT: u16 = 0;
    let host = crate::agent_recovery::federation_harness().await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let base = root.path().canonicalize().unwrap();
    let directory = base.join(DIRECTORY);
    let changed_directory = base.join(CHANGED_DIRECTORY);
    std::fs::create_dir(&directory).unwrap();
    std::fs::create_dir(&changed_directory).unwrap();
    std::fs::write(directory.join(FILE_NAME), CONTENTS).unwrap();
    std::fs::write(changed_directory.join(FILE_NAME), CONTENTS).unwrap();
    let state = Arc::new(HubState::open(&base.join(STATE), host.home.path()).unwrap());
    let config = Configuration {
        listen: Some(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), DYNAMIC_PORT)),
        ..Configuration::default()
    };
    let node = ManagedNode::start(&host, state, config).await;
    let target = launch_managed(&node, &host, NAME_A).await;
    let before = host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let initial = node.snapshot().await;
    let mut driver = Driver::start(&host, &node).await;
    driver.file_roots_from_root().await;
    driver.send(ENTER).await;
    driver
        .wait(|text| text.contains(ADD_FILE_ROOT) && text.contains("Directory:"))
        .await;
    driver.field(ROOT_NAME).await;
    driver.field(directory.to_str().unwrap()).await;
    driver.send(ENTER).await;
    driver
        .wait(|text| text.contains(DRAFT) && text.contains(ADD_FILE_ROOT))
        .await;
    assert_eq!(node.snapshot().await.state, initial.state);
    driver.root_apply().await;
    driver.active(initial.state.revision + 1).await;
    let created = node.snapshot().await;
    let configured = &created.state.configuration.file_roots[0];
    let id = configured.fields().root_id;
    assert_eq!(configured.fields().path, directory.to_str().unwrap());
    assert_eq!(configured.fields().name, ROOT_NAME);
    let roots = yolo_hub_file_client::list_roots(&mut node.client().await, node.hub, deadline())
        .await
        .unwrap();
    assert_eq!(roots.roots[0].fields().target.address.root_id, id);
    let request = |target| {
        ReadFileRequest::new(ReadFileFields {
            schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
            target,
            relative_path: RelativeFilePath::new(FILE_NAME.into()).unwrap(),
            offset: 0,
            length: CONTENTS.len() as u32,
            expected_version: None,
        })
        .unwrap()
    };
    assert_eq!(
        yolo_hub_file_client::read(
            &mut node.client().await,
            request(roots.roots[0].fields().target),
            deadline()
        )
        .await
        .unwrap()
        .fields()
        .data,
        CONTENTS
    );
    driver.edit_root_path(base.to_str().unwrap()).await;
    driver.root_apply().await;
    driver.wait(|text| text.contains(REJECTED)).await;
    assert_eq!(node.snapshot().await.state, created.state);
    driver.reload_root_draft(created.state.revision).await;
    driver
        .edit_root_path(changed_directory.to_str().unwrap())
        .await;
    let concurrent = node.configure(created.state.configuration.clone()).await;
    driver.root_apply().await;
    driver.wait(|text| text.contains(CONFLICT)).await;
    driver.apply_from_root().await;
    assert_eq!(node.snapshot().await.state, concurrent.state);
    driver.reload_root_draft(concurrent.state.revision).await;
    driver
        .edit_root_path(changed_directory.to_str().unwrap())
        .await;
    driver.root_apply().await;
    driver.active(concurrent.state.revision + 1).await;
    let edited = node.snapshot().await;
    assert_eq!(
        edited.state.configuration.file_roots[0].fields().root_id,
        id
    );
    assert_eq!(
        edited.state.configuration.file_roots[0].fields().path,
        changed_directory.to_str().unwrap()
    );
    let roots = yolo_hub_file_client::list_roots(&mut node.client().await, node.hub, deadline())
        .await
        .unwrap();
    let removed_target = roots.roots[0].fields().target;
    driver.file_roots_from_root().await;
    driver.send(ENTER).await;
    driver.wait(|text| text.contains(EDIT_FILE_ROOT)).await;
    for _ in 0..REMOVE_FROM_NAME_STEPS {
        driver.send(TAB).await;
    }
    driver.send(ENTER).await;
    driver.wait(|text| text.contains(REMOVE_FILE_ROOT)).await;
    driver.send(ENTER).await;
    assert_eq!(node.snapshot().await.state, edited.state);
    driver.send(ENTER).await;
    driver.send(TAB).await;
    driver.send(ENTER).await;
    driver
        .wait(|text| text.contains(DRAFT) && text.contains(ADD_FILE_ROOT))
        .await;
    assert_eq!(node.snapshot().await.state, edited.state);
    driver.root_apply().await;
    driver.active(edited.state.revision + 1).await;
    assert!(
        node.snapshot()
            .await
            .state
            .configuration
            .file_roots
            .is_empty()
    );
    assert!(
        yolo_hub_file_client::read(
            &mut node.client().await,
            request(removed_target),
            deadline()
        )
        .await
        .is_err()
    );
    let after = host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    assert_eq!(
        after.runtime.as_ref().unwrap().runtime_epoch,
        before.runtime.as_ref().unwrap().runtime_epoch
    );
    assert_eq!(
        after.agent.as_ref().unwrap().process_pid,
        before.agent.as_ref().unwrap().process_pid
    );
    assert_eq!(
        after.agent.as_ref().unwrap().process_start_time,
        before.agent.as_ref().unwrap().process_start_time
    );
    driver.finish().await;
    node.stop().await;
}
