use super::{
    constants::*,
    fixture::{Fixture, empty_catalogue, large_catalogue},
};
use crate::size::{self, Counter};
use std::io::Write;
use yolo_hub_federation_directory::ObservationHistory;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol_test_support::support::Identity;
use yolo_paths::constants::MAX_DURABLE_JSON_BYTES;

#[test]
fn serializer_budget_accepts_exact_limit_and_rejects_overflow_without_buffering() {
    let bytes = INVALID_BYTES;
    let mut counter = Counter {
        remaining: bytes.len() as u64,
    };
    counter.write_all(bytes).unwrap();
    assert_eq!(counter.remaining, 0);
    counter.flush().unwrap();
    assert!(counter.write_all(&bytes[..1]).is_err());
    size::validate(&vec![0_u8; 8]).unwrap();
    assert!(size::validate(&DIRECTORY_FILL.repeat(MAX_DURABLE_JSON_BYTES as usize)).is_err());
}

#[test]
fn serializer_flush_never_replenishes_budget_or_charges_failed_writes() {
    let payload = INVALID_BYTES;
    let mut counter = Counter {
        remaining: payload.len() as u64,
    };
    counter.write_all(&payload[..1]).unwrap();
    let remaining = counter.remaining;
    counter.flush().unwrap();
    assert_eq!(counter.remaining, remaining);
    assert!(counter.write_all(payload).is_err());
    assert_eq!(counter.remaining, remaining);
    counter.flush().unwrap();
    counter.write_all(&payload[1..]).unwrap();
    assert_eq!(counter.remaining, 0);
    counter.flush().unwrap();
    assert!(counter.write_all(&payload[..1]).is_err());
}

#[test]
fn bad_entry_oversized_origin_and_union_capacity_do_not_poison_healthy_cache() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    store.record_history(&fixture.history()).unwrap();
    let bytes = std::fs::read(fixture.index_path()).unwrap();
    let index = fixture.version(&fixture.index_path());
    let foreign =
        ObservationHistory::new(Identity::new().identity, vec![], routing_limits()).unwrap();
    assert!(store.record_history(&foreign).is_err());
    let other = Identity::new();
    let huge = fixture.with(vec![large_catalogue(&other, OVERSIZED_SESSIONS)]);
    assert!(store.record_history(&huge).is_err());
    store.limits.max_known_agents = std::num::NonZeroUsize::new(ONE_ORIGIN).unwrap();
    assert!(
        store
            .record_history(&fixture.with(vec![empty_catalogue(&other)]))
            .is_err()
    );
    assert_eq!(store.history().unwrap(), fixture.history());
    assert_eq!(std::fs::read(fixture.index_path()).unwrap(), bytes);
    assert_eq!(fixture.version(&fixture.index_path()), index);
    assert!(!fixture.record_path(other.identity.hub()).exists());
}

#[test]
fn equal_revision_equivocation_is_nonmutating_and_lower_snapshots_cannot_rollback() {
    use yolo_hub_peer_protocol::{Purpose, Signed};
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let newer = fixture.revised(STEP + STEP, STEP + STEP);
    let latest = fixture.with(vec![newer.clone()]);
    store.record_history(&latest).unwrap();
    let record_path = fixture.record_path(fixture.peer.origin.identity.hub());
    let before = fixture.version(&record_path);
    let mut conflict = newer.payload;
    conflict.terminals.clear();
    let conflict = Signed::new(conflict, Purpose::Catalogue, &fixture.peer.origin).unwrap();
    assert!(store.record_history(&fixture.with(vec![conflict])).is_err());
    for catalogue in [
        fixture.peer.export.catalogue.clone(),
        fixture.revised(STEP + STEP + STEP, STEP),
    ] {
        store
            .record_history(&fixture.with(vec![catalogue]))
            .unwrap();
        assert_eq!(store.history().unwrap(), latest);
        assert_eq!(fixture.version(&record_path), before);
    }
}
