use super::probe_recovery_tests::ProbeFixture;
use super::*;

const HOOK_ERROR: &str = "injected probe cleanup fault";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fault {
    AfterQuarantine,
    AfterUnlink,
    BeforeQuarantine,
    BeforeUnlink,
    DirectoryScan,
}

#[test]
fn probe_cleanup_fault_matrix_preserves_every_unproven_boundary() {
    for fault in [
        Fault::AfterQuarantine,
        Fault::AfterUnlink,
        Fault::BeforeQuarantine,
        Fault::BeforeUnlink,
        Fault::DirectoryScan,
    ] {
        run_fault(fault);
    }
}

fn run_fault(fault: Fault) {
    let fixture = ProbeFixture::new();
    if fault == Fault::DirectoryScan {
        crate::paths::directory_stream::fail_next_stream_open_for_test();
    }
    let result = socket_stage::cleanup_key_with_all_test_hooks(
        &fixture.directory,
        fixture.target.key(),
        fixture.directory.path(),
        || fail_at(fault, Fault::BeforeQuarantine),
        || fail_at(fault, Fault::AfterQuarantine),
        || fail_at(fault, Fault::BeforeUnlink),
        || fail_at(fault, Fault::AfterUnlink),
    );
    assert!(result.is_err(), "{fault:?}: {result:?}");
    match fault {
        Fault::BeforeQuarantine | Fault::DirectoryScan => {
            assert!(fixture.probe.exists());
            assert!(!fixture.quarantine.exists());
        }
        Fault::AfterQuarantine | Fault::BeforeUnlink => {
            assert!(!fixture.probe.exists());
            assert!(fixture.quarantine.exists());
        }
        Fault::AfterUnlink => {
            assert!(!fixture.probe.exists());
            assert!(!fixture.quarantine.exists());
        }
    }
    assert!(fixture.lease.exists());
    let retry = socket_stage::cleanup_key(&fixture.directory, fixture.target.key()).unwrap();
    assert!(
        matches!(retry, socket_stage::Cleanup::Mutated),
        "{fault:?}: {retry:?}"
    );
}

fn fail_at(actual: Fault, expected: Fault) -> anyhow::Result<()> {
    if actual == expected {
        anyhow::bail!(HOOK_ERROR);
    }
    Ok(())
}
