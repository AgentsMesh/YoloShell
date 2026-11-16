use super::{DROP_RETURN_DEADLINE, assert_reaped, observe_completion};
use crate::worker::startup::startup_guard::tests::{TEST_MARKER_FILE, production_fixture};
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

const INJECTED_PRE_DESCRIPTOR_PANIC: &str =
    "injected panic before startup process descriptor installation";
const ROOT_CAPTURE_REQUIRED_DETAIL: &str = "resource checkpoint did not expose the startup root";
const COMPLETION_CAPTURE_REQUIRED_DETAIL: &str =
    "resource checkpoint did not expose fault-reaper completion";
const CLEANUP_SCOPE_CAPTURE_REQUIRED_DETAIL: &str =
    "resource checkpoint did not expose the startup cleanup scope";
const DROP_START_CAPTURE_REQUIRED_DETAIL: &str =
    "resource checkpoint did not capture the injected panic boundary";

#[tokio::test]
async fn panic_before_descriptor_install_closes_master_and_settles_cleanup() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let root = RefCell::new(None);
    let completion = RefCell::new(None);
    let cleanup_scope = RefCell::new(None);
    let started = RefCell::new(None);

    let panic = catch_unwind(AssertUnwindSafe(|| {
        production_fixture::spawn_with_after_capture(&marker, |resources, scope| {
            root.replace(Some(resources.child().identity().unwrap()));
            completion.replace(Some(observe_completion(resources.child_mut())));
            cleanup_scope.replace(Some(std::mem::replace(
                scope,
                crate::worker::StartupCleanupScope::new().unwrap(),
            )));
            started.replace(Some(Instant::now()));
            panic!("{INJECTED_PRE_DESCRIPTOR_PANIC}");
        });
    }));

    assert!(panic.is_err());
    let started = started
        .into_inner()
        .expect(DROP_START_CAPTURE_REQUIRED_DETAIL);
    assert!(started.elapsed() < DROP_RETURN_DEADLINE);
    let root = root.into_inner().expect(ROOT_CAPTURE_REQUIRED_DETAIL);
    assert_reaped(
        completion
            .into_inner()
            .expect(COMPLETION_CAPTURE_REQUIRED_DETAIL),
    );
    cleanup_scope
        .into_inner()
        .expect(CLEANUP_SCOPE_CAPTURE_REQUIRED_DETAIL)
        .settle()
        .await
        .unwrap();
    assert!(!root.matches());
}
