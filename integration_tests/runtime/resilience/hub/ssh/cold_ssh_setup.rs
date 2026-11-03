#[path = "cold_ssh_setup/constants.rs"]
mod constants;
#[path = "cold_ssh_setup/evidence.rs"]
mod evidence;
#[path = "cold_ssh_setup/fixture.rs"]
mod fixture;
#[path = "cold_ssh_setup/shell.rs"]
mod shell;
#[path = "cold_ssh_setup/terminal.rs"]
mod terminal;
use super::{
    local_startup_support::retain_fixture_host, multipath::io, relay_crash::process::RelayProcess,
};
use crate::ssh_transport::SshdFixture;
use fixture::Fixture;
use yolo_hub_remote_setup_protocol::{SetupAction, SetupCheckpoint};

/// macOS localhost SSH exercises real publication/bootstrap; Linux acceptance is separate.
#[tokio::test(flavor = "multi_thread")]
async fn cold_ssh_setup_publishes_bootstraps_links_and_inspects_after_entry_restart_without_replay()
{
    let sshd = SshdFixture::start().await;
    let entry_host = crate::agent_recovery::federation_harness().await;
    let entry_hold = retain_fixture_host(&entry_host).await;
    let mut fixture = Fixture::new(&entry_host, &sshd).await;
    let entry = RelayProcess::start_existing(
        &entry_host,
        &fixture.entry_state,
        &fixture.entry_socket,
        fixture.entry_peer,
    )
    .await;
    fixture.register(&entry).await;
    let request = fixture.request(&entry).await;
    let operation_id = request.request.fields().request_id;
    let absent = entry
        .client()
        .await
        .inspect_remote_setup(operation_id, constants::deadline())
        .await
        .unwrap();
    assert_eq!(absent.owner, fixture.entry_peer.hub());
    assert!(absent.checkpoint.is_none());
    assert!(!fixture.install.exists());
    assert!(!fixture.remote_state.exists());
    let linked = fixture.setup(&entry, &request, SetupAction::Start).await;
    fixture.broker.capture().await;
    let receipt = evidence::enrollment(&linked);
    let before = evidence::Evidence::capture(&fixture, &request, &receipt).await;
    let mut shell = shell::Shell::create(&entry, &fixture, receipt.fields().peer).await;
    let mut first = io::attach_client(entry.client().await, shell.target, None).await;
    let cursor = terminal::exchange(&entry, shell.target, &mut first).await;
    shell.assert_survives(&fixture).await;
    let broker_before = fixture.broker.status().await;
    entry.stop().await;
    io::expect_transport_loss(&mut first, shell.target).await;
    drop(first);
    shell.assert_survives(&fixture).await;
    before.verify(&fixture, &request, &receipt).await;
    let entry = RelayProcess::start_existing(
        &entry_host,
        &fixture.entry_state,
        &fixture.entry_socket,
        fixture.entry_peer,
    )
    .await;
    let history = entry
        .client()
        .await
        .inspect_remote_setup(operation_id, constants::deadline())
        .await
        .unwrap();
    assert_eq!(history.operation_id, operation_id);
    assert_eq!(history.owner, fixture.entry_peer.hub());
    assert_eq!(
        history.checkpoint,
        Some(SetupCheckpoint::Complete {
            origin: receipt.fields().peer.hub(),
        })
    );
    before.verify(&fixture, &request, &receipt).await;
    evidence::wait_route(&entry, shell.target).await;
    let inspected = fixture.setup(&entry, &request, SetupAction::Inspect).await;
    assert_eq!(evidence::enrollment(&inspected), receipt);
    before.verify(&fixture, &request, &receipt).await;
    let broker_after = fixture.broker.status().await;
    assert_eq!(broker_before.instance_id, broker_after.instance_id);
    assert_eq!(broker_before.connection_id, broker_after.connection_id);
    assert_eq!(
        broker_before.connection_generation,
        broker_after.connection_generation
    );
    let mut restored = io::attach_client(entry.client().await, shell.target, Some(cursor)).await;
    terminal::exchange(&entry, shell.target, &mut restored).await;
    drop(restored);
    shell.assert_survives(&fixture).await;
    shell.stop(&entry).await;
    entry.stop().await;
    fixture.finish().await;
    drop(entry_hold);
}
