use super::{fixtures::*, parent};
use yolo_hub_enrollment_store::{Admission, ENROLLMENT_DIRECTORY, claim, inspect};
use yolo_remote_provisioning_protocol::RemoteBootstrapRequest;

#[test]
fn readonly_absence_creates_nothing_and_reopened_prepared_never_returns_a_submit_permit() {
    let (_root, parent) = parent();
    let prepared = prepared(false);
    assert!(
        inspect(&parent, prepared.peer, &prepared.request)
            .unwrap()
            .is_none()
    );
    assert!(!parent.path().join(ENROLLMENT_DIRECTORY).exists());
    let Admission::Fresh(fresh) = claim(
        parent.try_clone().unwrap(),
        prepared.peer,
        &prepared.request,
    )
    .unwrap() else {
        panic!()
    };
    let permit = fresh.prepare(prepared.clone()).unwrap();
    permit.verify().unwrap();
    drop(permit); // Crash after durable prepared, before Configure.
    let Admission::Existing(observed) = claim(
        parent.try_clone().unwrap(),
        prepared.peer,
        &prepared.request,
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(observed.record().prepared, prepared);
    assert!(observed.record().completion.is_none());
    observed.verify().unwrap();
    let mut wrong = prepared.request.fields().clone();
    wrong.remaining_reexports += 1;
    assert!(
        claim(
            parent,
            prepared.peer,
            &RemoteBootstrapRequest::new(wrong).unwrap()
        )
        .is_err()
    );
}

#[test]
fn completed_receipt_is_durable_and_remains_bound_to_its_original_preparation() {
    for no_change in [false, true] {
        let (_root, parent) = parent();
        let prepared = prepared(no_change);
        let Admission::Fresh(claim) = claim(
            parent.try_clone().unwrap(),
            prepared.peer,
            &prepared.request,
        )
        .unwrap() else {
            panic!()
        };
        let receipt = completed(&prepared);
        let recorded = claim
            .prepare(prepared.clone())
            .unwrap()
            .complete(receipt.clone())
            .unwrap();
        assert_eq!(recorded.completion, Some(receipt));
        let observed = inspect(&parent, prepared.peer, &prepared.request)
            .unwrap()
            .unwrap();
        assert_eq!(observed.record(), &recorded);
        observed.verify().unwrap();
    }
}
