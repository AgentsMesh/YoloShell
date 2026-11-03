use super::*;
use crate::model::TerminalManifest;
use crate::pty_custody::{CustodyDurableState, CustodyRetirementEvidence};
use std::cell::Cell;

const SETTLEMENT_FAILURE: &str = "prepared provider settlement interrupted";

#[path = "../../test_support/custody_retirement/prepared_test_custody.rs"]
mod custody;
#[path = "../../test_support/custody_retirement/prepared_test_records.rs"]
mod records;
#[path = "../../test_support/custody_retirement/prepared_test_support.rs"]
mod support;
use support::Fixture;

#[test]
fn exact_prepared_candidate_publishes_bound_retirement_receipt() {
    let fixture = Fixture::new();

    publish_exact(
        &fixture.namespace,
        &fixture.paths,
        &fixture.custody,
        &fixture.prepared,
        fixture.prepared_identity,
        fixture.candidate,
        |_| Ok(()),
    )
    .unwrap();

    let retired = fixture.current().0;
    assert!(matches!(retired.state, CustodyDurableState::Retired { .. }));
    assert_eq!(retired.retired_lease(), Some(fixture.candidate));
    assert_eq!(
        retired.retired_lease().unwrap().guardian,
        fixture.candidate.guardian
    );
    assert_eq!(retired.retirement_evidence(), Some(fixture.evidence));
    let evidence: CustodyRetirementEvidence = retired.retirement_evidence().unwrap();
    assert_eq!(
        evidence.controller_generation(),
        fixture.candidate.controller_generation
    );
    assert_eq!(evidence.runtime_file_identity(), fixture.runtime_identity);
    let manifest: TerminalManifest = fixture
        .namespace
        .read_json(&fixture.paths.manifest(fixture.prepared.runtime.terminal_id))
        .unwrap();
    assert!(!manifest.archived);
}

#[test]
fn same_value_inode_aba_rejects_stale_prepared_identity() {
    let fixture = Fixture::new();
    let replacement = fixture.replace_same_value();
    assert_eq!(replacement.0, fixture.prepared);
    assert_ne!(replacement.1, fixture.prepared_identity);

    assert!(
        publish_exact(
            &fixture.namespace,
            &fixture.paths,
            &fixture.custody,
            &fixture.prepared,
            fixture.prepared_identity,
            fixture.candidate,
            |_| Ok(()),
        )
        .is_err()
    );
    assert_eq!(fixture.current(), replacement);
}

#[test]
fn exact_retired_retry_is_idempotent() {
    let fixture = Fixture::new();
    let committed_identity = Cell::new(None);

    publish_exact_observed(
        &fixture.namespace,
        &fixture.paths,
        &fixture.custody,
        &fixture.prepared,
        fixture.prepared_identity,
        fixture.candidate,
        |_| Ok(()),
        |staged_identity| {
            let committed = fixture.current();
            assert!(committed.1.same_object(staged_identity));
            assert_eq!(committed.0.retired_lease(), Some(fixture.candidate));
            assert_eq!(committed.0.retirement_evidence(), Some(fixture.evidence));
            assert!(committed_identity.replace(Some(committed.1)).is_none());
        },
    )
    .unwrap();
    let committed = fixture.current();
    assert_eq!(committed_identity.get(), Some(committed.1));
    assert_eq!(committed.0.retired_lease(), Some(fixture.candidate));
    assert_eq!(committed.0.retirement_evidence(), Some(fixture.evidence));

    retry_retired(&fixture).unwrap();
    assert_eq!(fixture.current(), committed);
}

#[test]
fn same_value_inode_aba_after_parent_sync_fails_exact_retired_retry() {
    let fixture = Fixture::new();
    publish_exact(
        &fixture.namespace,
        &fixture.paths,
        &fixture.custody,
        &fixture.prepared,
        fixture.prepared_identity,
        fixture.candidate,
        |_| Ok(()),
    )
    .unwrap();
    let committed = fixture.current();
    let path = fixture
        .paths
        .pty_custody(fixture.prepared.runtime.terminal_id);
    let replacement = committed.0.clone();
    let hook = super::super::test_hook::install_after_receipt_parent_sync(move || {
        crate::paths::atomic_write_json(&path, &replacement)
    });

    assert!(retry_retired(&fixture).is_err());
    drop(hook);
    let replaced = fixture.current();
    assert_eq!(replaced.0, committed.0);
    assert_ne!(replaced.1, committed.1);

    retry_retired(&fixture).unwrap();
    assert_eq!(fixture.current(), replaced);
}

fn retry_retired(fixture: &Fixture) -> anyhow::Result<()> {
    super::super::publish_exact(
        &fixture.namespace,
        &fixture.paths,
        &fixture.custody,
        &fixture.prepared.runtime,
        fixture.candidate,
    )
}

#[test]
fn prepared_retirement_waits_for_provider_settlement_before_publication() {
    let fixture = Fixture::new();
    let before = fixture.current();
    assert!(
        publish_exact(
            &fixture.namespace,
            &fixture.paths,
            &fixture.custody,
            &fixture.prepared,
            fixture.prepared_identity,
            fixture.candidate,
            |authority| {
                authority.verify()?;
                assert_eq!(fixture.current(), before);
                anyhow::bail!(SETTLEMENT_FAILURE)
            },
        )
        .is_err()
    );
    assert_eq!(fixture.current(), before);
}
