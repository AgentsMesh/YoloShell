use super::super::{AuthorityProcessIdentity, source_before_reboot_migration};
use super::support::Fixture;

const SOURCE_BOOT: &str = "claude-generation-source-boot";
const TARGET_BOOT: &str = "claude-generation-target-boot";
const FOREIGN_BOOT: &str = "claude-generation-foreign-boot";
const WORKER_INCREMENT: u32 = 1;

#[test]
fn reboot_migration_restores_only_exact_runtime_boot_identity() {
    let fixture = Fixture::new();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let mut target = fixture.store.load_set(&fixture.lease).unwrap().unwrap();
    target.authorities[0].handshake_boot_id = TARGET_BOOT.to_owned();
    let source = source_before_reboot_migration(
        &target,
        fixture.begin.terminal_id,
        fixture.begin.runtime_epoch,
        fixture.begin.worker,
        SOURCE_BOOT,
        TARGET_BOOT,
    )
    .unwrap();
    assert_eq!(source.authorities[0].handshake_boot_id, SOURCE_BOOT);
}

#[test]
fn reboot_migration_rejects_foreign_boot_and_worker_identity() {
    let fixture = Fixture::new();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let mut target = fixture.store.load_set(&fixture.lease).unwrap().unwrap();
    target.authorities[0].handshake_boot_id = FOREIGN_BOOT.to_owned();
    assert!(migrate(&fixture, &target, fixture.begin.worker).is_err());
    target.authorities[0].handshake_boot_id = TARGET_BOOT.to_owned();
    let foreign_worker = AuthorityProcessIdentity {
        pid: fixture.begin.worker.pid + WORKER_INCREMENT,
        ..fixture.begin.worker
    };
    assert!(migrate(&fixture, &target, foreign_worker).is_err());
}

fn migrate(
    fixture: &Fixture,
    target: &super::super::ClaudeInvocationAuthorities,
    worker: AuthorityProcessIdentity,
) -> anyhow::Result<super::super::ClaudeInvocationAuthorities> {
    source_before_reboot_migration(
        target,
        fixture.begin.terminal_id,
        fixture.begin.runtime_epoch,
        worker,
        SOURCE_BOOT,
        TARGET_BOOT,
    )
}
