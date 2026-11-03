use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Barrier};

const FIRST_THREAD_DETAIL: &str = "first inventory thread";
const SECOND_THREAD_DETAIL: &str = "second inventory thread";

#[test]
fn rejected_nested_install_preserves_the_current_fixture() {
    let _current = install_test_inventory([Vec::new()]);

    let nested = catch_unwind(AssertUnwindSafe(|| install_test_inventory([Vec::new()])));

    assert!(nested.is_err());
    assert!(take_test_inventory().unwrap().unwrap().is_empty());
}

#[test]
fn dropped_guard_allows_a_new_fixture_on_the_same_thread() {
    drop(install_test_inventory([Vec::new()]));

    let _current = install_test_inventory([Vec::new()]);

    assert!(take_test_inventory().unwrap().unwrap().is_empty());
}

#[test]
fn concurrent_threads_observe_only_their_own_fixture() {
    let barrier = Arc::new(Barrier::new(2));
    let first = inventory_thread(Arc::clone(&barrier), FIRST_THREAD_DETAIL);
    let second = inventory_thread(barrier, SECOND_THREAD_DETAIL);

    assert_eq!(first.join().unwrap(), FIRST_THREAD_DETAIL);
    assert_eq!(second.join().unwrap(), SECOND_THREAD_DETAIL);
}

fn inventory_thread(
    barrier: Arc<Barrier>,
    detail: &'static str,
) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let failure = std::io::Error::other(detail);
        let _fixture = install_test_inventory_results([Err(failure)]);
        barrier.wait();
        take_test_inventory().unwrap().unwrap_err().to_string()
    })
}
