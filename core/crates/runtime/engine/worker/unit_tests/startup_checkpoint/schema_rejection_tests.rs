use super::*;

#[test]
fn unsupported_checkpoints_cannot_be_replayed_settled_or_rewritten() {
    let fixture = CheckpointFixture::new();
    ActiveStartupCheckpoint::begin(
        &fixture.namespace,
        &fixture.paths,
        &fixture.launch,
        gone_process(),
    )
    .unwrap();
    let mut unsupported = fixture.read();
    for version in 0..constants::STARTUP_CHECKPOINT_SCHEMA_VERSION {
        unsupported.schema_version = version;
        crate::paths::atomic_write_json(&fixture.path(), &unsupported).unwrap();
        let before = std::fs::read(fixture.path()).unwrap();
        assert!(validation::validate(&unsupported).is_err());
        assert!(
            worker_startup_checkpoint_replay_disposition(&fixture.paths, &fixture.launch,).is_err()
        );
        assert!(
            settle_worker_startup_checkpoint_for_launch(
                &fixture.paths,
                &fixture.launch,
                WorkerStartupCheckpointPolicy::SettlePresent,
            )
            .is_err()
        );
        assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    }
}
