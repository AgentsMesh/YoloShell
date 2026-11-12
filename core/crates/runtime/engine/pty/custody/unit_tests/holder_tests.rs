use super::*;
use crate::process::{ProcessIdentityRelation, install_test_process_identity_relation};

#[test]
fn holder_observation_preserves_all_process_identity_states() {
    let holder = CustodyHolderIdentity::capture().unwrap();
    assert_eq!(holder.observation(), CustodyHolderObservation::Current);

    let gone = install_test_process_identity_relation(
        holder.pid,
        holder.start_time,
        ProcessIdentityRelation::Gone,
    );
    assert_eq!(holder.observation(), CustodyHolderObservation::ProvenGone);
    drop(gone);

    let unknown = install_test_process_identity_relation(
        holder.pid,
        holder.start_time,
        ProcessIdentityRelation::Unknown,
    );
    assert_eq!(
        holder.observation(),
        CustodyHolderObservation::Indeterminate
    );
    assert!(!holder.matches());
    drop(unknown);

    assert_eq!(holder.observation(), CustodyHolderObservation::Current);
}
