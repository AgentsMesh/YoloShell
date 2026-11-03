use super::*;
use crate::worker::codex_hook::projection::test_support::{ProjectionFixture, registry};

const AUTHORITY_REVISION: u64 = 7;

#[test]
fn retrying_one_lifecycle_version_reuses_the_exact_binding_payload() {
    let fixture = ProjectionFixture::new();
    let route = fixture.route();
    let lifecycle = fixture.start();
    let first = project(
        &route,
        &lifecycle,
        AUTHORITY_REVISION,
        ProjectionMode::Active,
        None,
        &registry(),
    )
    .unwrap();

    let replay = project(
        &route,
        &lifecycle,
        AUTHORITY_REVISION,
        ProjectionMode::Active,
        Some(&first.binding),
        &registry(),
    )
    .unwrap();

    assert_eq!(replay.version, first.version);
    assert_eq!(replay.status, first.status);
    assert_eq!(replay.binding, first.binding);
}
