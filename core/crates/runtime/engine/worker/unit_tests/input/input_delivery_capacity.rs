use crate::worker::tests::*;
use std::sync::atomic::AtomicUsize;

const CAPACITY_HOLDER: &str = "capacity-holder";
const CAPACITY_FILLER_INPUT: &[u8] = b"capacity-filler";

#[tokio::test]
async fn distinct_capability_pressure_evicts_the_oldest_capability() {
    let fixture = Fixture::new();
    let controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(controller, &fixture.lease_request(CAPACITY_HOLDER), false)
        .await
        .unwrap();
    let writes = Arc::new(AtomicUsize::new(0));
    *fixture.state.writer.lock().unwrap() = Some(PtyInputWriter::test(SharedInputWriter {
        bytes: Arc::new(Mutex::new(Vec::new())),
        writes: Arc::clone(&writes),
        fail: false,
    }));
    let oldest = WriterCapability::generate();
    let oldest_input = Uuid::new_v4();
    let mut newest = oldest.clone();
    let mut newest_input = oldest_input;
    {
        let mut cache = fixture.state.delivered_inputs.lock().unwrap();
        cache.record(
            &oldest,
            oldest_input,
            CAPACITY_FILLER_INPUT,
            CachedInputDelivery::Delivered,
            DELIVERED_INPUT_CACHE_LIMIT,
        );
        for _ in 1..DELIVERED_INPUT_CACHE_LIMIT {
            newest = WriterCapability::generate();
            newest_input = Uuid::new_v4();
            cache.record(
                &newest,
                newest_input,
                CAPACITY_FILLER_INPUT,
                CachedInputDelivery::Delivered,
                DELIVERED_INPUT_CACHE_LIMIT,
            );
        }
        assert_eq!(cache.len(), DELIVERED_INPUT_CACHE_LIMIT);
        assert_eq!(cache.capability_len(), DELIVERED_INPUT_CACHE_LIMIT);
        assert_eq!(cache.evictable_len(), 0);
    }

    let input = capability_input_request(&fixture, &lease, CAPABILITY_INPUT);
    assert!(
        !fixture
            .state
            .write_input(controller, &input)
            .await
            .unwrap()
            .duplicate
    );
    assert_eq!(writes.load(Ordering::Acquire), 1);
    {
        let cache = fixture.state.delivered_inputs.lock().unwrap();
        assert_eq!(
            cache.get(&oldest, oldest_input, CAPACITY_FILLER_INPUT),
            CachedInputLookup::Missing,
            "pressure retained the oldest capability"
        );
        assert_eq!(
            cache.get(&newest, newest_input, CAPACITY_FILLER_INPUT),
            CachedInputLookup::Matching(CachedInputDelivery::Delivered),
            "pressure evicted a capability newer than the oldest"
        );
        assert_eq!(
            cache.get(&lease.writer_capability, input.input_id, &input.data),
            CachedInputLookup::Matching(CachedInputDelivery::Delivered)
        );
        assert_eq!(cache.len(), DELIVERED_INPUT_CACHE_LIMIT);
        assert_eq!(cache.capability_len(), DELIVERED_INPUT_CACHE_LIMIT);
    }

    // Eviction loses the dedup record, not delivery: a retry of the evicted
    // capability is written again instead of being mistaken for a duplicate.
    {
        let mut cache = fixture.state.delivered_inputs.lock().unwrap();
        cache.record(
            &oldest,
            oldest_input,
            CAPACITY_FILLER_INPUT,
            CachedInputDelivery::Delivered,
            DELIVERED_INPUT_CACHE_LIMIT,
        );
        assert_eq!(
            cache.get(&oldest, oldest_input, CAPACITY_FILLER_INPUT),
            CachedInputLookup::Matching(CachedInputDelivery::Delivered)
        );
        assert_eq!(
            cache.get(&newest, newest_input, CAPACITY_FILLER_INPUT),
            CachedInputLookup::Matching(CachedInputDelivery::Delivered),
            "re-recording the evicted capability removed the newest one"
        );
    }
}
