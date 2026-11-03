use super::*;
use crate::paths::identity::FileIdentity;
use crate::process::{ProcessIdentityRelation, ProcessIdentityRelationGuard};
use crate::pty_custody::{
    CustodyDurableState, CustodyGuardianBinding, CustodyGuardianPreparation, CustodyHolderIdentity,
    CustodyLeaseId, CustodyOffer, CustodyRequest,
};
use std::cell::Cell;
use uuid::Uuid;

#[path = "../../test_support/replacement_test_support.rs"]
mod support;
use support::Fixture;

const SUPERSESSION_GENERATION_INCREMENT: u64 = 1;
const ABANDONED_GUARDIAN_PID: u32 = 72_001;
const ABANDONED_GUARDIAN_START: u64 = 82_001;
const SUCCESSOR_HOST_PID: u32 = 72_002;
const SUCCESSOR_HOST_START: u64 = 82_002;
const SUCCESSOR_GUARDIAN_PID: u32 = 72_003;
const SUCCESSOR_GUARDIAN_START: u64 = 82_003;

#[test]
fn initial_install_commit_observer_reconciles_post_rename_parent_sync_failure() {
    let fixture = Fixture::new(ProcessIdentityRelation::Gone);
    let predecessor = CustodyDurableRecord::pending(fixture.request.runtime.clone());
    let predecessor_identity = write_record(&fixture, &predecessor);
    let binding = fixture.binding();
    let expected = predecessor
        .prepare_guardian(&fixture.request, &fixture.offer, binding)
        .unwrap();
    let commit_observed = Cell::new(None);

    let preparation = install_observed(
        &fixture.namespace,
        &fixture.paths,
        &predecessor,
        predecessor_identity,
        fixture.runtime_identity,
        &fixture.request,
        &fixture.offer,
        binding,
        inject_parent_sync_failure(&commit_observed),
    )
    .unwrap();

    assert_eq!(
        commit_observed.get(),
        Some(preparation.custody_file_identity())
    );
    assert_exact_installation(
        &fixture,
        &expected,
        predecessor_identity,
        binding,
        preparation,
    );
}

#[test]
fn successor_supersession_commit_observer_reconciles_post_rename_parent_sync_failure() {
    let fixture = Fixture::new(ProcessIdentityRelation::Gone);
    let abandoned_guardian = holder(ABANDONED_GUARDIAN_PID, ABANDONED_GUARDIAN_START);
    let abandoned_binding = CustodyGuardianBinding::new(
        Uuid::new_v4(),
        abandoned_guardian,
        &fixture.request,
        &fixture.offer,
    )
    .unwrap();
    let predecessor = fixture
        .snapshot
        .record()
        .prepare_guardian(&fixture.request, &fixture.offer, abandoned_binding)
        .unwrap();
    let predecessor_identity = write_record(&fixture, &predecessor);
    let (request, offer, binding) = successor(&fixture);
    let expected = predecessor
        .supersede_prepared_guardian(&request, &offer, binding)
        .unwrap();
    let _abandoned_host = relation(fixture.request.holder(), ProcessIdentityRelation::Gone);
    let _abandoned_guardian = relation(abandoned_guardian, ProcessIdentityRelation::Gone);
    let _successor_guardian = relation(binding.process(), ProcessIdentityRelation::Matching);
    let commit_observed = Cell::new(None);

    let preparation = install_successor_observed(
        &fixture.namespace,
        &fixture.paths,
        &predecessor,
        predecessor_identity,
        fixture.runtime_identity,
        &request,
        &offer,
        binding,
        None,
        inject_parent_sync_failure(&commit_observed),
    )
    .unwrap();

    assert_eq!(
        commit_observed.get(),
        Some(preparation.custody_file_identity())
    );
    assert_exact_installation(
        &fixture,
        &expected,
        predecessor_identity,
        binding,
        preparation,
    );
}

fn successor(fixture: &Fixture) -> (CustodyRequest, CustodyOffer, CustodyGuardianBinding) {
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        fixture.request.runtime.clone(),
        holder(SUCCESSOR_HOST_PID, SUCCESSOR_HOST_START),
        fixture.request.auth_token().to_owned(),
        fixture
            .request
            .controller_generation
            .checked_add(SUPERSESSION_GENERATION_INCREMENT)
            .unwrap(),
    );
    let offer = CustodyOffer::new_with_profile(
        &request,
        fixture.offer.descriptor_set,
        fixture.offer.profile,
        fixture.offer.recovery_identity_sha256,
    )
    .unwrap();
    let binding = CustodyGuardianBinding::new(
        Uuid::new_v4(),
        holder(SUCCESSOR_GUARDIAN_PID, SUCCESSOR_GUARDIAN_START),
        &request,
        &offer,
    )
    .unwrap();
    (request, offer, binding)
}

fn write_record(fixture: &Fixture, record: &CustodyDurableRecord) -> FileIdentity {
    let path = fixture.paths.pty_custody(fixture.runtime.terminal_id);
    crate::paths::atomic_write_json(&path, record).unwrap();
    let current = fixture.current();
    assert_eq!(current.0, *record);
    current.1
}

fn assert_exact_installation(
    fixture: &Fixture,
    expected: &CustodyDurableRecord,
    predecessor_identity: FileIdentity,
    binding: CustodyGuardianBinding,
    preparation: CustodyGuardianPreparation,
) {
    let (durable, custody_identity) = fixture.current();
    assert_eq!(durable, *expected);
    assert_eq!(durable.runtime, fixture.request.runtime);
    let CustodyDurableState::PreparedGuardian { candidate, .. } = durable.state else {
        unreachable!();
    };
    assert!(binding.matches_lease_identity(&candidate));
    assert_eq!(preparation.binding(), binding);
    assert_eq!(
        preparation.runtime_file_identity(),
        fixture.runtime_identity
    );
    assert_eq!(
        preparation.predecessor_file_identity(),
        predecessor_identity
    );
    assert_eq!(preparation.custody_file_identity(), custody_identity);
    assert!(!custody_identity.same_object(predecessor_identity));
}

fn holder(pid: u32, start_time: u64) -> CustodyHolderIdentity {
    CustodyHolderIdentity { pid, start_time }
}

fn relation(
    holder: CustodyHolderIdentity,
    relation: ProcessIdentityRelation,
) -> ProcessIdentityRelationGuard {
    crate::process::install_test_process_identity_relation(holder.pid, holder.start_time, relation)
}

fn inject_parent_sync_failure(
    observed: &Cell<Option<FileIdentity>>,
) -> impl FnMut(FileIdentity) + '_ {
    move |identity| {
        assert!(observed.replace(Some(identity)).is_none());
        crate::paths::test_support::fail_next_directory_sync();
    }
}
