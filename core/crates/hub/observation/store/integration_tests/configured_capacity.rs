use crate::{constants::*, fixture::*};
use std::num::NonZeroUsize;
use yolo_hub_federation_directory::ObservationHistory;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_observation_store::ObservationStore;
use yolo_hub_peer_protocol::MAX_DIRECTORY_ORIGINS;
use yolo_hub_peer_protocol_test_support::support::Identity;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn configured_history_larger_than_one_peer_page_roundtrips_across_origin_shards() {
    let fixture = Fixture::new();
    let count = MAX_DIRECTORY_ORIGINS + ONE_ORIGIN;
    let mut limits = routing_limits();
    limits.max_known_agents = NonZeroUsize::new(count).unwrap();
    let catalogues = (0..count)
        .map(|_| empty_catalogue(&Identity::new()))
        .collect();
    let history = ObservationHistory::new(fixture.peer.entry.identity, catalogues, limits).unwrap();
    let mut store = ObservationStore::open(
        PrivateDirectory::open(&fixture.path).unwrap(),
        fixture.peer.entry.identity,
        limits,
    )
    .unwrap();
    store.record_history(&history).unwrap();
    assert_eq!(store.history().unwrap(), history);
    drop(store);
    let reopened = ObservationStore::open(
        PrivateDirectory::open(&fixture.path).unwrap(),
        fixture.peer.entry.identity,
        limits,
    )
    .unwrap();
    assert_eq!(reopened.history().unwrap(), history);
    assert_eq!(history.catalogues().len(), count);
}
