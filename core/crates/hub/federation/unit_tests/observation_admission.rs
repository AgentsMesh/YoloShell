use super::*;

#[test]
fn ownership_is_exclusive_for_one_hub_and_independent_between_hubs() {
    let first = HubId::new();
    let other = HubId::new();
    let owner = Admission::acquire(first).unwrap();
    assert!(Admission::acquire(first).is_none());
    assert!(Admission::acquire(first).is_none());
    let independent = Admission::acquire(other).unwrap();
    drop(owner);
    let successor = Admission::acquire(first).unwrap();
    assert!(Admission::acquire(other).is_none());
    drop((successor, independent));
}

#[tokio::test]
async fn a_panicking_worker_releases_admission_before_announcing_completion() {
    let hub = HubId::new();
    let admission = Admission::acquire(hub).unwrap();
    let mailbox = std::sync::Arc::new(crate::observation_mailbox::Mailbox::default());
    let (ready, _loaded) = tokio::sync::oneshot::channel();
    let (finished, mut closed) = tokio::sync::watch::channel(false);
    let worker = std::thread::spawn(move || {
        crate::observation_worker::run(
            |_| panic!("simulated worker initialization panic"),
            mailbox,
            ready,
            finished,
            admission,
        );
    });
    tokio::time::timeout(crate::constants::STOP_TIMEOUT, async {
        while !*closed.borrow_and_update() {
            closed.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    let successor = Admission::acquire(hub).unwrap();
    assert!(worker.join().is_err());
    drop(successor);
}
