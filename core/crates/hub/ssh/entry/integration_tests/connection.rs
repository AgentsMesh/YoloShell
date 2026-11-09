use crate::support::*;
use std::sync::{Arc, atomic::Ordering};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};
use yolo_hub_client::{EntryConnector, HubEntryStream};

async fn roundtrip(stream: &mut HubEntryStream) {
    stream.write_all(PAYLOAD).await.unwrap();
    let mut data = vec![u8::default(); PAYLOAD.len()];
    timeout_at(deadline(), stream.read_exact(&mut data))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(data, PAYLOAD);
}

#[tokio::test]
async fn concurrent_rpc_and_terminal_channels_reuse_one_outer_connection_and_one_forward() {
    let fixture = Fixture::start(Service::Echo).await;
    let entry = Arc::new(fixture.entry());
    let hub = fixture.profile.hub.hub_id;
    let (control, catalog, terminal) = tokio::join!(
        entry.open(hub, deadline()),
        entry.open(hub, deadline()),
        entry.open(hub, deadline())
    );
    let (mut control, mut catalog, mut terminal) =
        (control.unwrap(), catalog.unwrap(), terminal.unwrap());
    tokio::join!(
        roundtrip(&mut control),
        roundtrip(&mut catalog),
        roundtrip(&mut terminal)
    );
    drop(catalog);
    roundtrip(&mut control).await;
    roundtrip(&mut terminal).await;
    let mut query = entry.open(hub, deadline()).await.unwrap();
    roundtrip(&mut query).await;
    assert_eq!(fixture.counts.connections.load(Ordering::SeqCst), COUNT_ONE);
    assert_eq!(fixture.counts.forwards.load(Ordering::SeqCst), COUNT_ONE);
    assert_eq!(entry.client_id(), fixture.profile.client_id);
    entry.close(deadline()).await.unwrap();
    assert!(
        timeout_at(deadline(), terminal.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    assert!(entry.open(hub, deadline()).await.is_err());
}

#[tokio::test]
async fn dropping_entry_closes_retained_channels_even_while_callers_keep_streams() {
    let fixture = Fixture::start(Service::Echo).await;
    let entry = fixture.entry();
    let mut channel = entry
        .open(fixture.profile.hub.hub_id, deadline())
        .await
        .unwrap();
    roundtrip(&mut channel).await;
    drop(entry);
    assert!(
        timeout_at(deadline(), channel.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    timeout_at(deadline(), async {
        while fixture.counts.closed.load(Ordering::SeqCst) == NO_COUNT {
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}
