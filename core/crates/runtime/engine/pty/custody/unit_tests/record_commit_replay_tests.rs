use super::commit_support::*;
use crate::pty_custody::*;
use crate::sha256_digest::Sha256Digest;
use uuid::Uuid;

#[test]
fn valid_reconnections_cannot_adopt_another_lease_offer_or_controller_commit() {
    for profile in [
        CustodyProfile::ActiveRunning,
        CustodyProfile::ActiveTerminalReadOnly,
    ] {
        let (request, offer, record) = transaction(profile);
        assert_committed(&record, &request, &offer);
        let mut different_lease = request.clone();
        different_lease.lease_id = CustodyLeaseId::new();
        let mut matching_offer = offer.clone();
        matching_offer.lease_id = different_lease.lease_id;
        assert_other_transaction_rejected(&record, &different_lease, &matching_offer);
        let mut different_offer = offer.clone();
        different_offer.offer_id = CustodyOfferId::new();
        assert_other_transaction_rejected(&record, &request, &different_offer);
        let mut next_controller = request.clone();
        next_controller.controller_generation += GENERATION_STEP;
        assert_other_transaction_rejected(&record, &next_controller, &offer);
        let mut reused_holder_pid = request.clone();
        reused_holder_pid.holder.start_time += IDENTITY_STEP;
        assert_other_transaction_rejected(&record, &reused_holder_pid, &offer);
        assert_committed(&record, &request, &offer);
    }
}

#[test]
fn valid_runtime_replacements_never_borrow_the_previous_commit_receipt() {
    let changes: &[fn(&mut CustodyRuntimeIdentity)] = &[
        |runtime| runtime.runtime_epoch = Uuid::new_v4(),
        |runtime| runtime.worker_start_time += IDENTITY_STEP,
        |runtime| runtime.child_start_time += IDENTITY_STEP,
        |runtime| runtime.boot_id = UNKNOWN_BOOT.into(),
    ];
    for profile in [
        CustodyProfile::ActiveRunning,
        CustodyProfile::ActiveTerminalReadOnly,
    ] {
        let (request, offer, record) = transaction(profile);
        for change in changes {
            let mut request = request.clone();
            change(&mut request.runtime);
            let mut paired = offer.clone();
            paired.runtime = request.runtime.clone();
            assert_other_transaction_rejected(&record, &request, &paired);
        }
    }
}

#[test]
fn descriptor_attestation_profile_and_recovery_content_remain_bound_to_commit() {
    for profile in [
        CustodyProfile::ActiveRunning,
        CustodyProfile::ActiveTerminalReadOnly,
    ] {
        let (request, offer, record) = transaction(profile);
        let mut read_only = offer.clone();
        read_only.profile = match profile {
            CustodyProfile::ActiveRunning => CustodyProfile::ActiveTerminalReadOnly,
            CustodyProfile::ActiveTerminalReadOnly => CustodyProfile::ActiveRunning,
            CustodyProfile::ReleasedTerminal => unreachable!(),
        };
        assert_other_transaction_rejected(&record, &request, &read_only);
        let mut different_recovery = offer.clone();
        different_recovery.recovery_identity_sha256 = Sha256Digest::repeated(DIGEST_BYTE);
        assert_other_transaction_rejected(&record, &request, &different_recovery);
    }
    let (request, offer, record) = transaction(CustodyProfile::ActiveRunning);
    let mut changed_descriptors = offer.clone();
    changed_descriptors.descriptor_set =
        CustodyDescriptorSet::macos_process_capability(SECOND_READER_HANDLE, SECOND_WRITER_HANDLE);
    assert_other_transaction_rejected(&record, &request, &changed_descriptors);
    changed_descriptors.descriptor_set = CustodyDescriptorSet::LinuxContainment;
    assert_other_transaction_rejected(&record, &request, &changed_descriptors);
}

#[test]
fn guardian_must_be_the_exact_committed_instance_even_for_the_same_valid_offer() {
    let (request, offer, record) = transaction(CustodyProfile::ActiveRunning);
    let replacement =
        CustodyGuardianBinding::new(Uuid::new_v4(), request.holder(), &request, &offer).unwrap();
    assert!(replacement.matches(&request, &offer));
    assert_ne!(Some(replacement), record.armed_lease().unwrap().guardian);
    assert!(!record.matches_committed_offer(&request, &offer, Some(replacement)));
    assert!(!record.matches_committed_offer(&request, &offer, None));
}

#[test]
fn valid_pending_prepared_and_retired_documents_do_not_acknowledge_an_armed_commit() {
    let (request, offer, record) = transaction(CustodyProfile::ActiveRunning);
    let guardian = record.armed_lease().unwrap().guardian.unwrap();
    let pending = CustodyDurableRecord::pending(request.runtime.clone());
    let prepared = pending
        .prepare_guardian(&request, &offer, guardian)
        .unwrap();
    let retired = record.retire(record.armed_lease().unwrap()).unwrap();
    for document in [pending, prepared, retired] {
        assert!(document.is_valid());
        assert!(!document.matches_committed_offer(&request, &offer, Some(guardian)));
    }
}
