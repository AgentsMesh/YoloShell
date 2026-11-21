#[path = "offline_inventory/constants.rs"]
mod constants;
#[path = "offline_inventory/observations.rs"]
mod observations;
#[path = "offline_inventory/persisted.rs"]
mod persisted;
#[path = "offline_inventory/persisted_probe.rs"]
mod persisted_probe;
#[path = "offline_inventory/persisted_types.rs"]
mod persisted_types;
#[path = "offline_inventory/topology.rs"]
mod topology;
use crate::hub::{
    federation_agents::{launch, launch_client},
    multipath::{io, observe},
};
use constants::*;
use topology::Topology;

#[ignore = "real relay process needs an SSH profile to consume the origin after ADR 0036; ssh_profile_live covers the managed broker path"]
#[tokio::test(flavor = "multi_thread")]
async fn entry_sigkill_restores_offline_transitive_inventory_without_restoring_control_authority() {
    let topology = Topology::new().await;
    let relay = topology.relay.start(&topology.relay_host).await;
    let entry = topology.entry.start(&topology.entry_host).await;
    let hub = topology.entry.peer.hub();
    let local = launch_client(entry.client().await, hub, &topology.entry_host, ENTRY_AGENT).await;
    let relayed = launch_client(
        relay.client().await,
        topology.relay.peer.hub(),
        &topology.relay_host,
        RELAY_AGENT,
    )
    .await;
    let remote = launch(&topology.origin, &topology.origin_host, ORIGIN_AGENT).await;
    let before = observations::agents(&entry, hub, false).await;
    let local_custody = topology
        .entry_host
        .client()
        .get_session(local.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let relay_custody = topology
        .relay_host
        .client()
        .get_session(relayed.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let origin_custody = topology
        .origin_host
        .client()
        .get_session(remote.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let mut output = io::attach_client(entry.client().await, remote, None).await;
    let cursor = io::exchange_client(entry.client().await, remote, &mut output).await;
    persisted::wait(
        &topology.entry,
        &[
            topology.relay.peer,
            topology.origin.node.source().unwrap().signer,
        ],
        &before,
    )
    .await;
    entry.crash().await;
    io::expect_transport_loss(&mut output, remote).await;
    drop(output);
    relay.crash().await;
    let entry = topology.entry.reopen(&topology.entry_host).await;
    let restored = observations::agents(&entry, hub, true).await;
    for (before, after) in before.iter().zip(&restored) {
        assert_eq!(before.cursor().target(), after.cursor().target());
        assert_eq!(before.fields().terminal, after.fields().terminal);
        assert_eq!(
            before.fields().working_directory,
            after.fields().working_directory
        );
    }
    observations::offline(&entry, hub, &[relayed, remote]).await;
    observe::custody(&topology.entry_host, local, &local_custody).await;
    observe::custody(&topology.relay_host, relayed, &relay_custody).await;
    observe::custody(&topology.origin_host, remote, &origin_custody).await;
    let mut local_output = io::attach_client(entry.client().await, local, None).await;
    io::exchange_client(entry.client().await, local, &mut local_output).await;
    drop(local_output);
    let relay = topology.relay.reopen(&topology.relay_host).await;
    let reconnected = observations::agents(&entry, hub, false).await;
    for (before, after) in before.iter().zip(reconnected) {
        assert_eq!(before.cursor().target(), after.cursor().target());
        assert_eq!(before.fields().terminal, after.fields().terminal);
    }
    let mut output = io::attach_client(entry.client().await, remote, Some(cursor)).await;
    io::exchange_client(entry.client().await, remote, &mut output).await;
    drop(output);
    observe::custody(&topology.origin_host, remote, &origin_custody).await;
    entry.stop().await;
    relay.stop().await;
    topology.finish().await;
}
