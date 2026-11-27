use crate::{exports::*, fixtures::*, history_support::catalogue, support::*};
use std::num::NonZeroUsize;
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::{FIRST_REVISION, MAX_DIRECTORY_ORIGINS};

#[test]
fn configured_history_capacity_can_retain_more_than_one_protocol_batch_across_install_and_restore()
{
    let fixture = Fixture::new();
    let origin_count = MAX_DIRECTORY_ORIGINS + ONE_PATH;
    let origins: Vec<_> = (0..origin_count).map(|_| Identity::new()).collect();
    let mut limits = routing_limits();
    limits.max_known_agents = NonZeroUsize::new(origin_count).unwrap();
    let pins = std::iter::once(fixture.relay.identity)
        .chain(origins.iter().map(|origin| origin.identity))
        .collect();
    let mut book = OriginBook::new(fixture.entry.identity, pins, limits).unwrap();
    let exports: Vec<_> = origins
        .iter()
        .map(|origin| {
            export_via(
                origin,
                &fixture.entry,
                &[&fixture.relay],
                catalogue(origin),
                observe(),
            )
        })
        .collect();
    let (first_batch, next_batch) = exports.split_at(MAX_DIRECTORY_ORIGINS);
    let link = install(&mut book, &fixture.relay, first_batch.to_vec());
    book.install(
        fixture.relay.identity,
        link,
        FIRST_REVISION + STEP,
        next_batch.to_vec(),
        NOW,
    )
    .unwrap();
    let retained = book.history();
    assert_eq!(retained.catalogues().len(), origin_count);
    retained.validate(limits).unwrap();
    assert_exports(&book, next_batch.to_vec(), NOW);
    let mut restored = OriginBook::new(fixture.entry.identity, Vec::new(), limits).unwrap();
    restored.restore_history(retained.clone()).unwrap();
    assert_eq!(restored.history(), retained);
    assert_eq!(restored.hubs(NOW).unwrap().len(), origin_count);
    assert!(restored.exportable(NOW).unwrap().is_empty());
}
