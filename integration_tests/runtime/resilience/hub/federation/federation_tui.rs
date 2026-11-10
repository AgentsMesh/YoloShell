use super::federation_support::*;
use crate::{Harness, runtime_ref_from_view};
#[path = "federation_tui_attachment.rs"]
mod attachment;
#[path = "federation_tui_capture.rs"]
mod capture;
#[path = "federation_tui_inventory.rs"]
mod inventory;
#[path = "federation_tui_layout.rs"]
mod layout;
#[path = "federation_tui_lease.rs"]
mod lease;
#[path = "federation_tui_workspace.rs"]
mod persistence;
use capture::{ping, wait_screen};
#[path = "federation_tui/constants.rs"]
mod constants;
use constants::*;
pub async fn exercise(
    node: &RunningNode,
    harness: &Harness,
    visible: &[&str],
    hidden: &[&str],
    remote_owner: Option<&Harness>,
) {
    let binary = crate::bazel_runfile(&std::env::var(TUI_BINARY_ENV).unwrap()).unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let workspace = workspace.path().canonicalize().unwrap();
    let driver = harness.create_cat(DRIVER_NAME).await;
    let target = runtime_ref_from_view(&driver);
    let client = harness.client();
    let grant = client
        .acquire_writer(
            target,
            DRIVER_NAME,
            Some(yolo_runtime::lease::constants::MAX_TTL_MS),
        )
        .await
        .unwrap();
    let renewal = lease::Renewal::start(client.clone(), target, grant.clone());
    attachment::size(&client, target, &grant).await;
    let mut output = attachment::open(&client, harness, &driver, node.node.hub_id()).await;
    let snapshot = output.initial.snapshot.as_ref().unwrap();
    let mut screen = vt100::Parser::new(snapshot.rows, snapshot.cols, 0);
    screen.process(&snapshot.data);
    let (launch, complete) = capture::launch(&binary, node, &workspace);
    client
        .input(
            target,
            grant.generation,
            &grant.writer_capability,
            launch.as_bytes().to_vec(),
        )
        .await
        .unwrap();
    wait_screen(&mut output, &mut screen, |text| {
        visible.iter().all(|name| text.contains(name))
    })
    .await;
    let text = screen.screen().contents();
    for name in hidden {
        assert!(
            !text.contains(name),
            "unexpected origin session in TUI: {name}"
        );
    }
    let mut checkpoint = None;
    let mut restored_output = String::new();
    inventory::exercise(
        node,
        &client,
        target,
        &grant,
        &mut output,
        &mut screen,
        visible,
    )
    .await;
    if let Some(origin) = remote_owner {
        let select = format!("/{NAME_C}\r\r").into_bytes();
        client
            .input(target, grant.generation, &grant.writer_capability, select)
            .await
            .unwrap();
        let title = format!("{NAME_C}{CONTROL_LABEL}");
        wait_screen(&mut output, &mut screen, |text| text.contains(&title)).await;
        let paused = node.catalog_gate.pause();
        paused.started().await;
        let (command, marker) = ping();
        let text = std::str::from_utf8(&command)
            .unwrap()
            .strip_suffix('\r')
            .unwrap();
        let mut paste = PASTE_START.to_vec();
        paste.extend_from_slice(text.as_bytes());
        paste.extend_from_slice(PASTE_END);
        client
            .input(target, grant.generation, &grant.writer_capability, paste)
            .await
            .unwrap();
        client
            .input(
                target,
                grant.generation,
                &grant.writer_capability,
                ENTER.to_vec(),
            )
            .await
            .unwrap();
        wait_screen(&mut output, &mut screen, |text| text.contains(&marker)).await;
        drop(paused);
        restored_output = marker;
        client
            .input(
                target,
                grant.generation,
                &grant.writer_capability,
                SPLIT.to_vec(),
            )
            .await
            .unwrap();
        persistence::track_created(origin, &workspace, node.node.hub_id()).await;
        layout::exercise(
            &client,
            target,
            &grant,
            &mut output,
            &mut screen,
            &workspace,
        )
        .await;
        checkpoint = Some(persistence::checkpoint(&workspace, node.node.hub_id()));
    }
    client
        .input(
            target,
            grant.generation,
            &grant.writer_capability,
            QUIT.to_vec(),
        )
        .await
        .unwrap();
    wait_screen(&mut output, &mut screen, |text| text.contains(&complete)).await;
    if remote_owner.is_some() {
        let saved = persistence::checkpoint(&workspace, node.node.hub_id());
        assert_eq!(Some(&saved), checkpoint.as_ref());
        persistence::assert_passive(node, &saved).await;
        let (launch, complete) = capture::launch(&binary, node, &workspace);
        client
            .input(
                target,
                grant.generation,
                &grant.writer_capability,
                launch.as_bytes().to_vec(),
            )
            .await
            .unwrap();
        wait_screen(&mut output, &mut screen, |text| {
            text.contains(TERMINALS_HEADER)
        })
        .await;
        client
            .input(
                target,
                grant.generation,
                &grant.writer_capability,
                ESCAPE.to_vec(),
            )
            .await
            .unwrap();
        let restored_title = format!("{NAME_C}{RESTORED_ACCESS}");
        wait_screen(&mut output, &mut screen, |text| {
            text.contains(&restored_title) && text.contains(&restored_output)
        })
        .await;
        persistence::assert_passive(node, &saved).await;
        client
            .input(
                target,
                grant.generation,
                &grant.writer_capability,
                QUIT.to_vec(),
            )
            .await
            .unwrap();
        wait_screen(&mut output, &mut screen, |text| text.contains(&complete)).await;
        let restored = persistence::checkpoint(&workspace, node.node.hub_id());
        persistence::assert_restored(&restored, &saved);
    }
    let grant = renewal.stop().await;
    client.release_writer(&grant, target).await.unwrap();
    output.detach().await.unwrap();
    client
        .terminate(driver.manifest.terminal_id, false)
        .await
        .unwrap();
}
