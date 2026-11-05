use super::{constants::*, fixture::Fixture};
use std::sync::{Arc, Barrier};
use yolo_hub_remote_setup_store::{OpenedSetup, SetupOperation};

#[test]
fn competing_owners_cannot_both_admit_one_request_and_later_open_is_existing() {
    let fixture = Fixture::new();
    let begin = Arc::new(Barrier::new(THREADS));
    let finish = Arc::new(Barrier::new(THREADS));
    let admitted = std::thread::scope(|scope| {
        let tasks = (0..THREADS)
            .map(|_| {
                let parent = fixture.parent.try_clone().unwrap();
                let record = fixture.record.clone();
                let begin = begin.clone();
                let finish = finish.clone();
                scope.spawn(move || {
                    begin.wait();
                    let opened = SetupOperation::open(parent, record);
                    let fresh = matches!(&opened, Ok(OpenedSetup::Fresh(_)));
                    finish.wait();
                    drop(opened);
                    fresh
                })
            })
            .collect::<Vec<_>>();
        tasks
            .into_iter()
            .map(|task| task.join().unwrap())
            .filter(|fresh| *fresh)
            .count()
    });
    assert_eq!(admitted, ONE_OWNER);
    assert!(matches!(fixture.open(), OpenedSetup::Existing(_)));
}
