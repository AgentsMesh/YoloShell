use super::{constants::*, deadline, driver::Driver, fixture::Fixture, output, shell, workspace};
use crate::{Harness, hub::support::input};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{AttachRequest, LeaseMode};
use yolo_hub_terminal_client::Attachment;

#[tokio::test(flavor = "multi_thread")]
async fn remote_tui_creates_writes_restores_read_only_and_loses_authority_on_revocation_after_live_control()
 {
    let fixture = Fixture::start(&FULL_RIGHTS).await;
    let driver_host = Harness::start().await;
    let directory = fixture
        .root
        .path()
        .canonicalize()
        .unwrap()
        .join(CREATED_DIRECTORY);
    std::fs::create_dir(&directory).unwrap();
    let mut driver = Driver::start(&driver_host, &fixture, tempfile::tempdir().unwrap()).await;
    driver.send(NEW_SHELL).await;
    driver.wait(|text| text.contains(DIRECTORY_PROMPT)).await;
    driver.send(CLEAR_DEFAULT_DIRECTORY).await;
    driver.send(directory.to_str().unwrap().as_bytes()).await;
    driver.send(FINISH_INPUT).await;
    let saved = workspace::bound(&driver, &fixture).await;
    let address = workspace::binding(&saved).unwrap();
    assert_eq!(address.hub_id, fixture.hub);
    let original = fixture
        .host
        .client()
        .get_session(address.terminal_id.as_uuid())
        .await
        .unwrap();
    fixture.host.track_view(&original);
    assert_eq!(original.manifest.cwd, directory);
    let Reply::Terminal { terminal: session } = fixture
        .process
        .client()
        .await
        .call(fixture.hub, Command::Get { address }, deadline())
        .await
        .unwrap()
    else {
        panic!("remote TUI's shell is missing at its origin")
    };
    let target = session.fields().runtime.unwrap().target;
    let name = &session.fields().name;
    // A live remote creation enters control after a fresh Acquire.
    driver
        .wait_named("initial control", |text| {
            text.contains(&format!("{name}{CONTROL}"))
        })
        .await;
    let (command, first_marker) = shell::marker();
    driver.send(&command).await;
    driver
        .wait_named("initial marker", |text| text.contains(&first_marker))
        .await;
    let stored = driver.finish().await;
    fixture.no_writer(target).await;
    fixture.custody(&original).await;
    let mut restored = Driver::start(&driver_host, &fixture, stored).await;
    let read_only = format!("{name}{READ_ONLY}");
    restored
        .wait_named("restored read-only", |text| {
            text.contains(&read_only) && text.contains(&first_marker)
        })
        .await;
    assert_eq!(
        workspace::binding(&workspace::bound(&restored, &fixture).await),
        Some(address)
    );
    fixture.no_writer(target).await;
    restored.open(address, name).await;
    let (command, marker) = shell::marker();
    restored.send(&command).await;
    restored.wait(|text| text.contains(&marker)).await;
    let mut witness = Attachment::open(
        fixture.process.client().await,
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    fixture.configure(vec![]).await;
    restored
        .wait_named("revocation disconnect", |text| {
            text.contains(DISCONNECTED_HEADER)
        })
        .await;
    let (forbidden, forbidden_marker) = shell::marker();
    restored.send(&forbidden).await;
    // A following UI transition proves the TUI has consumed the preceding input.
    restored.send(INVENTORY).await;
    restored.send(SEARCH).await;
    restored.wait(|text| text.contains(SEARCH_STATUS)).await;
    restored.send(FINISH_INPUT).await;
    restored.wait(|text| !text.contains(SEARCH_STATUS)).await;
    fixture.custody(&original).await;
    // An independent local command supplies a causal output fence after rejected input.
    let mut local = fixture.process.client().await;
    let Reply::LeaseObservation { lease } = local
        .call(fixture.hub, Command::ObserveLease { target }, deadline())
        .await
        .unwrap()
    else {
        panic!("origin lease observation is missing")
    };
    let mode = lease.map_or(LeaseMode::Acquire {}, |lease| LeaseMode::TakeControl {
        observed_generation: lease.fields().generation,
    });
    let Reply::Lease { grant } = local
        .call(fixture.hub, fixture.lease_command(target, mode), deadline())
        .await
        .unwrap()
    else {
        panic!("independent local control is unavailable")
    };
    let (command, marker) = shell::marker();
    local
        .call(fixture.hub, input(grant, &command), deadline())
        .await
        .unwrap();
    let observed = output::until(&mut witness, &marker).await;
    assert!(!String::from_utf8_lossy(&observed).contains(&forbidden_marker));
    local
        .call(fixture.hub, Command::Release { grant }, deadline())
        .await
        .unwrap();
    restored.finish().await;
    fixture.custody(&original).await;
    assert_eq!(
        fixture
            .host
            .client()
            .list_sessions_complete(false)
            .await
            .unwrap()
            .len(),
        SHELL_COUNT
    );
    drop((local, witness));
    fixture.stop().await;
}
