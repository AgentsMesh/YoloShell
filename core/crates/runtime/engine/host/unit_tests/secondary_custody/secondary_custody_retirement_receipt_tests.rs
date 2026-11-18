use super::*;
use crate::pty_custody::{CustodyDurableRecord, CustodyDurableState, CustodyRuntimeIdentity};
#[path = "../../test_support/unit/secondary_custody/secondary_custody_retirement_receipt_test_support.rs"]
mod retirement_support;
use retirement_support::*;

const IDENTITY_INCREMENT: u64 = 1;
const RECEIPT_SOCKET_NAME: &str = "durable-retirement-receipt.sock";
const RETIRED_RECEIPT_REQUIRED_DETAIL: &str = "test requires a retired receipt";

#[tokio::test]
async fn empty_registry_accepts_only_an_exact_durable_retirement() {
    let fixture = ReopenedRetirement::new();

    assert_eq!(
        fixture.host.secondary_custody.recovery_admission(
            &fixture.host,
            &CustodyRuntimeIdentity::from_record(&fixture.runtime),
        ),
        SecondaryCustodyRecoveryAdmission::Retired
    );
}

#[tokio::test]
async fn durable_armed_predecessor_is_not_a_retirement_receipt() {
    let directory = tempdir().unwrap();
    let first = Host::open(crate::HostConfig::new(directory.path())).unwrap();
    let runtime = dead_runtime(&first, directory.path());
    first
        .secondary_custody
        .install_test_active_lease(&first, &runtime);
    drop(first);
    let reopened = Host::open(crate::HostConfig::new(directory.path())).unwrap();

    assert_eq!(
        reopened
            .secondary_custody
            .recovery_admission(&reopened, &CustodyRuntimeIdentity::from_record(&runtime),),
        SecondaryCustodyRecoveryAdmission::Unavailable
    );
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn live_receipt_holder_awaits_settlement_and_unknown_fails_closed() {
    let mut fixture = ReopenedRetirement::new_with_guardian_relation(
        crate::process::ProcessIdentityRelation::Matching,
    );
    let runtime = CustodyRuntimeIdentity::from_record(&fixture.runtime);
    assert_eq!(
        fixture
            .host
            .secondary_custody
            .recovery_admission(&fixture.host, &runtime),
        SecondaryCustodyRecoveryAdmission::AwaitingSettlement
    );
    fixture.set_guardian_relation(crate::process::ProcessIdentityRelation::Unknown);
    assert_eq!(
        fixture
            .host
            .secondary_custody
            .recovery_admission(&fixture.host, &runtime),
        SecondaryCustodyRecoveryAdmission::Unavailable
    );
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn guardian_exit_between_receipt_observations_never_becomes_unavailable() {
    use crate::pty_custody::CustodyHolderObservation;

    let fixture = ReopenedRetirement::new();
    let runtime = CustodyRuntimeIdentity::from_record(&fixture.runtime);
    let _observations = super::super::recovery_evidence::install_guardian_observations_for_test([
        CustodyHolderObservation::Current,
        CustodyHolderObservation::ProvenGone,
    ]);

    assert_eq!(
        fixture
            .host
            .secondary_custody
            .recovery_admission(&fixture.host, &runtime),
        SecondaryCustodyRecoveryAdmission::AwaitingSettlement
    );
}

#[tokio::test]
async fn stale_generation_or_runtime_identity_fails_closed() {
    let fixture = ReopenedRetirement::new();
    let path = fixture.host.paths.pty_custody(fixture.runtime.terminal_id);
    let mut receipt: CustodyDurableRecord = fixture.host.namespace.read_json(&path).unwrap();
    let CustodyDurableState::Retired {
        controller_generation,
        ..
    } = &mut receipt.state
    else {
        panic!("{RETIRED_RECEIPT_REQUIRED_DETAIL}");
    };
    *controller_generation = fixture.host.controller_generation;
    fixture.host.namespace.write_json(&path, &receipt).unwrap();
    let runtime = CustodyRuntimeIdentity::from_record(&fixture.runtime);
    assert_unavailable(&fixture.host, &runtime);

    let mut changed = runtime;
    changed.child_start_time = changed.child_start_time.saturating_add(IDENTITY_INCREMENT);
    assert_unavailable(&fixture.host, &changed);
}

#[tokio::test]
async fn evidence_rejects_a_same_value_file_replacement() {
    let fixture = ReopenedRetirement::new();
    let runtime = CustodyRuntimeIdentity::from_record(&fixture.runtime);
    let evidence = fixture
        .host
        .secondary_custody
        .retirement_evidence(&fixture.host, &runtime)
        .unwrap();
    let path = fixture.host.paths.pty_custody(fixture.runtime.terminal_id);
    let receipt: CustodyDurableRecord = fixture.host.namespace.read_json(&path).unwrap();
    fixture.host.namespace.write_json(&path, &receipt).unwrap();

    assert!(evidence.verify(&fixture.host).is_err());
}

#[tokio::test]
async fn same_value_runtime_replacement_fails_closed() {
    let fixture = ReopenedRetirement::new();
    let runtime = CustodyRuntimeIdentity::from_record(&fixture.runtime);
    let path = fixture
        .host
        .paths
        .runtime_record(fixture.runtime.terminal_id);
    fixture
        .host
        .namespace
        .write_json(&path, &fixture.runtime)
        .unwrap();

    assert_unavailable(&fixture.host, &runtime);
}

#[tokio::test]
async fn same_value_journal_or_cursor_replacement_fails_closed() {
    for cursor in [false, true] {
        let fixture = ReopenedRetirement::new();
        let runtime = CustodyRuntimeIdentity::from_record(&fixture.runtime);
        let journal = fixture.host.paths.journal(fixture.runtime.terminal_id);
        let path = if cursor {
            crate::journal::retained_cursor_path_for_test(&journal)
        } else {
            journal
        };
        replace_file_same_value(&path);

        assert_unavailable(&fixture.host, &runtime);
    }
}

fn assert_unavailable(host: &Host, runtime: &CustodyRuntimeIdentity) {
    assert_eq!(
        host.secondary_custody.recovery_admission(host, runtime),
        SecondaryCustodyRecoveryAdmission::Unavailable
    );
}
