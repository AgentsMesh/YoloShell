use crate::{CatalogueChanges, PeerDirectoryWatch, fixtures::Fixture, support::STEP};
use std::time::Duration;
use tokio::time::timeout;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_peer_protocol::{PEER_PROTOCOL_VERSION, PeerRequest};

const BUDGET: Duration = Duration::from_secs(2);
const LONG_WAIT_MS: u32 = 20_000;
const QUIET: Duration = Duration::from_millis(15);

#[tokio::test]
async fn only_committed_visible_changes_wake_clients_for_that_origin() {
    let f = Fixture::new();
    let local = f.entry.identity.hub();
    let peer = f.origin.identity.hub();
    let changes = CatalogueChanges::default();
    let cursor = |reply| match reply {
        Some(yolo_hub_rpc_protocol::Reply::CatalogueCursors {
            cursor, origins, ..
        }) => (cursor, origins),
        _ => panic!("expected catalogue cursors"),
    };
    let (initial, origins) = cursor(changes.wait(None, 0).await);
    assert!(origins.unwrap().is_empty());
    changes.touch([peer]);
    let (old, origins) = cursor(changes.wait(Some(initial), 0).await);
    let peer_version = origins.unwrap()[&peer];
    changes.revise([local], false);
    let wait = changes.wait(Some(old), LONG_WAIT_MS);
    tokio::pin!(wait);
    assert!(timeout(QUIET, &mut wait).await.is_err());
    changes.touch([local]);
    let (new, origins) = cursor(timeout(BUDGET, wait).await.unwrap());
    let origins = origins.unwrap();
    assert_ne!(new, old);
    assert_eq!(origins[&peer], peer_version);
    assert!(origins.contains_key(&local));
    assert_eq!(cursor(changes.wait(Some(old), 0).await).0, new);
    assert!(cursor(changes.wait(Some(new), 0).await).1.is_none());
    let retired = changes.wait(Some(new), LONG_WAIT_MS);
    tokio::pin!(retired);
    assert!(timeout(QUIET, &mut retired).await.is_err());
    changes.retire();
    assert!(timeout(BUDGET, retired).await.unwrap().is_none());
}

#[tokio::test]
async fn discovery_wakes_on_exportable_renewals_and_replays_changes_during_the_previous_read() {
    let f = Fixture::new();
    let local = f.origin.identity.hub();
    let peer = f.entry.identity.hub();
    let changes = CatalogueChanges::default();
    let watch = PeerDirectoryWatch::default();
    let config = Configuration::default();
    let request = PeerRequest::Discover {
        version: PEER_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        allowed_origins: vec![local, f.relay.identity.hub()],
    };
    let poll = || watch.before(&changes, &config, local, peer, &request);
    timeout(BUDGET, poll()).await.unwrap().unwrap();
    changes.touch([f.relay.identity.hub()]);
    let pending = poll();
    tokio::pin!(pending);
    assert!(timeout(QUIET, &mut pending).await.is_err());
    changes.revise([local], false);
    timeout(BUDGET, pending).await.unwrap().unwrap();
    // Publication arriving after sampling and before the response is sent is not lost.
    changes.touch([local]);
    timeout(BUDGET, poll()).await.unwrap().unwrap();
    let mut heartbeat = f.export.catalogue.payload.clone();
    heartbeat.revision += STEP;
    heartbeat.issued_at_ms += STEP;
    heartbeat.expires_at_ms += STEP;
    assert!(!crate::inventory_changed(
        Some(&f.export.catalogue.payload),
        &heartbeat
    ));
    heartbeat.agents.clear();
    heartbeat.terminals.clear();
    assert!(crate::inventory_changed(
        Some(&f.export.catalogue.payload),
        &heartbeat
    ));
}
