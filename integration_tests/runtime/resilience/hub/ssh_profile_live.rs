use super::{
    federation_agents::launch,
    federation_support::{POLL, RunningNode, TEMP_ROOT, deadline, exports, peer},
    local_startup_support::retain_fixture_host,
    multipath::{io, observe},
    relay_crash::process,
};
use crate::ssh_transport::SshdFixture;
use std::sync::Arc;
use yolo_hub_federation_settings::{Configuration, Peer, PeerAddress};
use yolo_hub_ssh_profile_protocol::SshProfileConfiguration;
use yolo_hub_state::HubState;
#[path = "ssh_profile_live_broker.rs"]
pub(super) mod broker;
#[path = "ssh_profile_live/constants.rs"]
mod constants;
#[path = "ssh_profile_live_observe.rs"]
pub(super) mod observations;
use constants::*;

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread")]
async fn real_hub_starts_detached_ssh_broker_and_reuses_it_after_entry_restart() {
    let sshd = SshdFixture::start().await;
    let entry_host = crate::agent_recovery::federation_harness().await;
    let entry_hold = retain_fixture_host(&entry_host).await;
    let origin_host = crate::agent_recovery::federation_harness().await;
    let origin_hold = retain_fixture_host(&origin_host).await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let base = root.path().canonicalize().unwrap();
    let entry_path = base.join(ENTRY_DIRECTORY);
    let entry_state = HubState::open(&entry_path, entry_host.home.path()).unwrap();
    let entry_peer = entry_state.peer_identity();
    let entry_socket = entry_state.socket_path();
    drop(entry_state);
    let origin_state =
        Arc::new(HubState::open(&base.join(ORIGIN_DIRECTORY), origin_host.home.path()).unwrap());
    let origin_peer = origin_state.peer_identity();
    let origin_peer_socket = origin_state.peer_socket().unwrap();
    let origin_config = Configuration {
        peers: vec![peer(
            entry_peer,
            None,
            false,
            vec![exports(origin_peer.hub(), false)],
        )],
        ..Default::default()
    };
    let origin =
        RunningNode::start(&origin_host, origin_state, origin_config).await;
    let target = launch(&origin, &origin_host, ORIGIN_AGENT).await;
    let expected_agent = *origin
        .wait_origins(&[origin_peer.hub()])
        .await
        .first()
        .unwrap();
    let before = origin_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let mut broker = broker::Broker::prepare(&base, origin_host.home.path(), &sshd);
    let entry_config = Configuration {
        peers: vec![Peer {
            identity: origin_peer,
            dial: Some(
                PeerAddress::profile(
                    broker.registration.profile.profile_id().to_owned(),
                    origin_peer_socket.clone(),
                )
                .unwrap(),
            ),
            consume: true,
            exports: vec![],
        }],
        ..Default::default()
    };
    let config_path = process::write_configuration(&entry_path, &Configuration::default());
    let entry = process::RelayProcess::start(
        &entry_host,
        &entry_path,
        &entry_socket,
        entry_peer,
        &config_path,
    )
    .await;
    assert!(
        observations::profiles(&entry, entry_peer.hub())
            .await
            .configuration
            .profiles
            .is_empty()
    );
    let profiles = SshProfileConfiguration {
        profiles: vec![broker.registration.clone()],
        ..Default::default()
    };
    let registered = observations::configure(&entry, entry_peer.hub(), profiles).await;
    let connected =
        observations::connect(&entry, entry_peer.hub(), &broker.registration.profile).await;
    let captured = broker.capture_if_ready().await;
    assert!(
        connected.is_ok(),
        "{CONNECT_FAILURE}: {connected:?}; {CAPTURE_DIAGNOSTIC}: {captured}; {BROKER_DIAGNOSTIC}: {:?}\n{ENTRY_DIAGNOSTIC}: {}\n{SSHD_DIAGNOSTIC}: {}",
        broker.client().status().await,
        entry.diagnostic(),
        sshd.diagnostics(),
    );
    if !captured {
        broker.capture().await;
    }
    let connected = connected.unwrap();
    observations::link(&entry, entry_peer.hub(), entry_config).await;
    let agent = observations::agent(&entry, entry_peer.hub(), target, true).await;
    assert_eq!(agent.cursor().target().address, expected_agent);
    let agent_target = agent.cursor().target();
    let mut first = io::attach_client(entry.client().await, target, None).await;
    let cursor = io::exchange_client(entry.client().await, target, &mut first).await;
    let broker_before = broker.status().await;
    assert!(broker_before.connection_id.is_some());
    assert!(broker_before.adopted);
    assert_eq!(connected.broker_instance_id, broker_before.instance_id);
    assert_eq!(Some(connected.connection_id), broker_before.connection_id);
    broker.assert_same_custody();
    entry.stop().await;
    io::expect_transport_loss(&mut first, target).await;
    drop(first);
    observe::custody(&origin_host, target, &before).await;
    broker.assert_same_custody();
    let entry =
        process::RelayProcess::start_existing(&entry_host, &entry_path, &entry_socket, entry_peer)
            .await;
    assert_eq!(
        observations::profiles(&entry, entry_peer.hub()).await,
        registered
    );
    observations::same_agent(
        &observations::agent(&entry, entry_peer.hub(), target, true).await,
        agent_target,
    );
    let broker_after = broker.status().await;
    assert_eq!(broker_after.instance_id, broker_before.instance_id);
    assert_eq!(broker_after.connection_id, broker_before.connection_id);
    assert_eq!(
        broker_after.connection_generation,
        broker_before.connection_generation
    );
    let mut restored = io::attach_client(entry.client().await, target, Some(cursor)).await;
    let cursor = io::exchange_client(entry.client().await, target, &mut restored).await;
    observations::configure(&entry, entry_peer.hub(), SshProfileConfiguration::default()).await;
    io::expect_transport_loss(&mut restored, target).await;
    drop(restored);
    observations::same_agent(
        &observations::agent(&entry, entry_peer.hub(), target, false).await,
        agent_target,
    );
    let broker_remaining = broker.status().await;
    assert_eq!(broker_remaining.instance_id, broker_before.instance_id);
    assert_eq!(broker_remaining.connection_id, broker_before.connection_id);
    assert_eq!(
        broker_remaining.connection_generation,
        broker_before.connection_generation
    );
    let mut direct = io::attach(&origin, target, Some(cursor)).await;
    io::exchange(&origin, target, &mut direct).await;
    drop(direct);
    observe::custody(&origin_host, target, &before).await;
    entry.stop().await;
    origin.stop().await;
    broker.stop().await;
    drop((entry_hold, origin_hold));
}
