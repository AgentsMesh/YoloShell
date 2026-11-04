use super::custody_rollover_tests::{pending, published_rollover_fixture, write_custody};

#[test]
fn preinstalled_retry_rejects_same_content_inode_replacement() {
    let state = published_rollover_fixture();
    let successor_pending = pending(&state.replacement);
    write_custody(&state, &successor_pending);
    let fence =
        super::super::super::secondary_custody_publication_fence::PublicationFence::capture(
            &state.fixture.namespace,
            &state.fixture.paths,
            &state.replacement,
            &successor_pending,
            &state.checkpoint,
        )
        .unwrap();
    write_custody(&state, &successor_pending);
    let retry = fence.already_installed(
        &state.fixture.namespace,
        &state.fixture.paths,
        &state.replacement,
        &successor_pending,
        &state.checkpoint,
    );
    assert!(retry.is_err());
}
