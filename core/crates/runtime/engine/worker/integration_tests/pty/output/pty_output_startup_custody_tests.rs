use super::*;
use crate::process::ProcessIdentityRelation;
use crate::worker::pty_output_progress::MasterAliasReleasePoll;
use std::sync::Arc;

#[test]
fn host_release_required_never_mints_a_local_only_closure_proof() {
    let mut output = startup_release_output();
    output
        .require_host_release(crate::pty_custody::CustodyHolderIdentity::capture().unwrap())
        .unwrap();
    output.mark_owner_master_released();
    output.close_local_aliases_for_host_release().unwrap();

    assert!(matches!(
        output.release_master_aliases().unwrap(),
        MasterAliasReleasePoll::Pending
    ));
    assert!(matches!(
        output.release_master_aliases().unwrap(),
        MasterAliasReleasePoll::Pending
    ));
}

#[test]
fn exact_host_release_unlocks_the_final_closure_proof_once() {
    let mut output = startup_release_output();
    output
        .require_host_release(crate::pty_custody::CustodyHolderIdentity::capture().unwrap())
        .unwrap();
    output.mark_owner_master_released();
    output.close_local_aliases_for_host_release().unwrap();
    output
        .prove_host_release(crate::pty_custody::ExactCustodyReleasedAck::for_test())
        .unwrap();

    assert!(matches!(
        output.release_master_aliases().unwrap(),
        MasterAliasReleasePoll::Closed(_)
    ));
    assert!(matches!(
        output.release_master_aliases().unwrap(),
        MasterAliasReleasePoll::Delivered
    ));
}

#[test]
fn ambiguous_descriptor_transfer_blocks_local_only_finalization() {
    let mut output = startup_release_output();
    let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
    let transfer = Arc::new(StartupCustodyTransferFence::new());
    output
        .bind_custody_transfer_fence(Arc::clone(&transfer))
        .unwrap();
    transfer.mark_transfer_possible(holder).unwrap();
    output.mark_owner_master_released();

    assert!(matches!(
        output.release_master_aliases().unwrap(),
        MasterAliasReleasePoll::Pending
    ));
}

#[test]
fn exact_holder_death_recovers_a_lost_release_ack() {
    let mut output = startup_release_output();
    let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
    output.require_host_release(holder).unwrap();
    output.mark_owner_master_released();
    output.close_local_aliases_for_host_release().unwrap();
    let _gone = crate::process::install_test_process_identity_relation(
        holder.pid,
        holder.start_time,
        ProcessIdentityRelation::Gone,
    );

    assert!(matches!(
        output.release_master_aliases().unwrap(),
        MasterAliasReleasePoll::Closed(_)
    ));
}

#[test]
fn failed_local_proof_acquisition_never_becomes_delivered() {
    let mut output = startup_release_output();
    output.mark_owner_master_released();
    output
        .master_alias_authority
        .as_ref()
        .unwrap()
        .take_local_aliases_closed()
        .unwrap();

    assert!(output.release_master_aliases().is_err());
    assert!(output.release_master_aliases().is_err());
}

fn startup_release_output() -> ExclusivePtyOutput {
    ExclusivePtyOutput::startup_custody_for_test()
}
