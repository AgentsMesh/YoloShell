use crate::worker::tests::*;
const STALE_RELEASE_GENERATION_INCREMENT: u64 = 1;
pub(super) const CAPABILITY_INPUT: &[u8] = b"capability-input\n";
const DISTINCT_CAPABILITY_INPUT: &[u8] = b"distinct-capability-input\n";
const RELEASED_WRITER_HOLDER: &str = "released-writer";
const RETAINED_WRITER_HOLDER: &str = "retained-writer";

#[test]
fn identical_input_ids_are_scoped_to_writer_capabilities() {
    let first = WriterCapability::generate();
    let second = WriterCapability::generate();
    let input_id = Uuid::new_v4();
    let mut cache = InputDeliveryCache::default();
    cache.record(
        &first,
        input_id,
        CAPABILITY_INPUT,
        CachedInputDelivery::Delivered,
        DELIVERED_INPUT_CACHE_LIMIT,
    );
    assert_eq!(
        cache.get(&second, input_id, DISTINCT_CAPABILITY_INPUT),
        CachedInputLookup::Missing
    );
    cache.record(
        &second,
        input_id,
        DISTINCT_CAPABILITY_INPUT,
        CachedInputDelivery::Unknown,
        DELIVERED_INPUT_CACHE_LIMIT,
    );
    assert_eq!(
        cache.get(&first, input_id, CAPABILITY_INPUT),
        CachedInputLookup::Matching(CachedInputDelivery::Delivered)
    );
    assert_eq!(
        cache.get(&second, input_id, DISTINCT_CAPABILITY_INPUT),
        CachedInputLookup::Matching(CachedInputDelivery::Unknown)
    );
}

#[tokio::test]
async fn successful_release_clears_all_capability_deliveries() {
    let fixture = Fixture::new();
    let state = &fixture.state;
    let controller = fixture.controller();
    let mut release = fixture.lease_request(RELEASED_WRITER_HOLDER);
    let lease = state
        .acquire_lease(controller, &release, false)
        .await
        .unwrap();
    let first = capability_input_request(&fixture, &lease, CAPABILITY_INPUT);
    let second = capability_input_request(&fixture, &lease, DISTINCT_CAPABILITY_INPUT);
    state.write_input(controller, &first).await.unwrap();
    state.write_input(controller, &second).await.unwrap();
    release.generation = Some(lease.generation);
    state.release_lease(controller, &release).await.unwrap();
    let cache = state.delivered_inputs.lock().unwrap();
    assert_eq!(
        cache.get(&lease.writer_capability, first.input_id, &first.data),
        CachedInputLookup::Missing
    );
    assert_eq!(
        cache.get(&lease.writer_capability, second.input_id, &second.data),
        CachedInputLookup::Missing
    );
}

#[tokio::test]
async fn failed_release_preserves_capability_deliveries() {
    let fixture = Fixture::new();
    let state = &fixture.state;
    let controller = fixture.controller();
    let mut release = fixture.lease_request(RETAINED_WRITER_HOLDER);
    let lease = state
        .acquire_lease(controller, &release, false)
        .await
        .unwrap();
    let input = capability_input_request(&fixture, &lease, CAPABILITY_INPUT);
    assert!(!state.write_input(controller, &input).await.unwrap().duplicate);
    release.generation = Some(
        lease
            .generation
            .checked_add(STALE_RELEASE_GENERATION_INCREMENT)
            .unwrap(),
    );
    assert!(state.release_lease(controller, &release).await.is_err());
    assert!(state.write_input(controller, &input).await.unwrap().duplicate);
}

pub(super) fn capability_input_request(
    fixture: &Fixture,
    lease: &WriterLeaseGrant,
    data: &[u8],
) -> InputRequest {
    InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability.clone(),
        input_id: Uuid::new_v4(),
        data: data.to_vec(),
    }
}
