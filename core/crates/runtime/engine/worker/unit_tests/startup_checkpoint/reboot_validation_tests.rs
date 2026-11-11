use super::*;

const SOURCE_BOOT: &str = "startup-checkpoint-source-boot";
const TARGET_BOOT: &str = "startup-checkpoint-target-boot";
const GENERATION_INCREMENT: u64 = 1;

#[test]
fn reboot_migration_restores_only_checkpoint_boot_identity() {
    let fixture = CheckpointFixture::new();
    let mut runtime = retirement_test_support::activated_runtime(&fixture);
    let mut target = fixture.read();
    target.boot_id = TARGET_BOOT.to_owned();
    runtime.boot_id = TARGET_BOOT.to_owned();
    let source = reboot_validation::source_before_reboot_migration(
        &target,
        target.terminal_id,
        target.runtime_epoch,
        target.worker_id,
        target.controller_generation,
        &runtime,
        SOURCE_BOOT,
        TARGET_BOOT,
    )
    .unwrap();
    assert_eq!(source.boot_id, SOURCE_BOOT);
}

#[test]
fn reboot_migration_rejects_unrelated_checkpoint_identity() {
    let fixture = CheckpointFixture::new();
    let mut runtime = retirement_test_support::activated_runtime(&fixture);
    let mut target = fixture.read();
    target.boot_id = TARGET_BOOT.to_owned();
    runtime.boot_id = TARGET_BOOT.to_owned();
    assert!(
        reboot_validation::source_before_reboot_migration(
            &target,
            target.terminal_id,
            target.runtime_epoch,
            target.worker_id,
            target.controller_generation + GENERATION_INCREMENT,
            &runtime,
            SOURCE_BOOT,
            TARGET_BOOT,
        )
        .is_err()
    );
}
