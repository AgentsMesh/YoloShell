use super::{
    constants::{EXPECTED_SUBMIT_OWNERS, THREADS},
    fixtures::*,
    parent,
};
use std::sync::{Arc, Barrier};
use yolo_hub_enrollment_store::{Admission, claim};

#[test]
fn simultaneous_same_request_claims_produce_exactly_one_live_submit_owner() {
    let (_root, parent) = parent();
    let prepared = prepared(false);
    let begin = Arc::new(Barrier::new(THREADS));
    let finish = Arc::new(Barrier::new(THREADS));
    let admitted = std::thread::scope(|scope| {
        let handles = (0..THREADS)
            .map(|_| {
                let parent = parent.try_clone().unwrap();
                let prepared = prepared.clone();
                let begin = begin.clone();
                let finish = finish.clone();
                scope.spawn(move || {
                    begin.wait();
                    let permit = match claim(parent, prepared.peer, &prepared.request) {
                        Ok(Admission::Fresh(claim)) => Some(claim.prepare(prepared).unwrap()),
                        Ok(Admission::Existing(_)) | Err(_) => None,
                    };
                    finish.wait();
                    permit.is_some()
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .filter(|fresh| *fresh)
            .count()
    });
    assert_eq!(admitted, EXPECTED_SUBMIT_OWNERS);
    assert!(matches!(
        claim(parent, prepared.peer, &prepared.request).unwrap(),
        Admission::Existing(_)
    ));
}
