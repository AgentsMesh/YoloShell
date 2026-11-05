use super::*;

#[test]
fn different_boot_proves_even_ambiguous_startup_custody_gone() {
    let fixture = CheckpointFixture::new();
    let mut checkpoint = fixture.begin();
    for stage in STARTUP_STAGES {
        checkpoint
            .enter(&fixture.namespace, &fixture.paths, stage)
            .unwrap();
        if stage == StartupStage::ChildProcess {
            break;
        }
        checkpoint
            .complete(&fixture.namespace, &fixture.paths, stage)
            .unwrap();
    }
    let mut evidence = fixture.read();
    evidence.boot_id = TEST_OTHER_BOOT_ID.into();

    super::super::settlement::require_checkpoint_custody_gone_on_boot(
        &evidence,
        TEST_CURRENT_BOOT_ID,
    )
    .unwrap();
    assert!(
        super::super::settlement::require_checkpoint_custody_gone_on_boot(
            &evidence,
            TEST_OTHER_BOOT_ID,
        )
        .is_err()
    );
}

#[test]
fn unknown_boot_identity_never_releases_process_custody() {
    let fixture = CheckpointFixture::new();
    let mut evidence = fixture.begin().evidence;
    evidence.boot_id = crate::process::unknown_boot_id_for_test();

    assert!(
        super::super::settlement::require_checkpoint_custody_gone_on_boot(
            &evidence,
            TEST_CURRENT_BOOT_ID,
        )
        .is_err()
    );
}
