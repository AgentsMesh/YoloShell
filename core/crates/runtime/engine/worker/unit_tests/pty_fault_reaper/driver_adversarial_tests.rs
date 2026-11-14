const REAPER_DRIVER_TEST_TIMEOUT: Duration = Duration::from_secs(1);
const PAYLOAD_DROP_PANIC_DETAIL: &str = "injected reaper payload Drop panic";

struct ReaperPanicOnDrop;

impl Drop for ReaperPanicOnDrop {
    fn drop(&mut self) {
        panic!("{PAYLOAD_DROP_PANIC_DETAIL}");
    }
}

#[test]
fn normal_driver_return_cannot_abandon_live_custody() {
    let (reaper, receiver, root) = driver_reaper();
    let mut returned = false;

    reaper.reap_with_driver(|ownership| {
        if !std::mem::replace(&mut returned, true) {
            return;
        }
        live_authority::reap_without_delay(ownership);
    });

    let outcome = receiver
        .receive_timeout(REAPER_DRIVER_TEST_TIMEOUT)
        .unwrap();
    assert!(outcome.status.is_some());
    assert!(format!("{:#}", outcome.fault.unwrap()).contains(FAULT_REAPER_EARLY_RETURN_DETAIL));
    assert_already_reaped(root);
}

#[test]
fn panic_payload_drop_cannot_abandon_live_custody() {
    let (reaper, receiver, root) = driver_reaper();
    let mut panicked = false;

    reaper.reap_with_driver(|ownership| {
        if !std::mem::replace(&mut panicked, true) {
            std::panic::panic_any(ReaperPanicOnDrop);
        }
        live_authority::reap_without_delay(ownership);
    });

    let outcome = receiver
        .receive_timeout(REAPER_DRIVER_TEST_TIMEOUT)
        .unwrap();
    assert!(outcome.status.is_some());
    let fault = format!("{:#}", outcome.fault.unwrap());
    assert!(fault.contains(FAULT_REAPER_PANIC_DETAIL));
    assert!(fault.contains(crate::worker::constants::TASK_PANIC_DETAIL));
    assert_already_reaped(root);
}

fn driver_reaper() -> (FaultReap, FaultReapReceiver, ProcessIdentity) {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    let reaper = active_reaper(authority, tree, completion);
    (reaper, receiver, root)
}

fn assert_already_reaped(root: ProcessIdentity) {
    let mut status = libc::c_int::default();
    let waited = unsafe { libc::waitpid(root.pid as libc::pid_t, &mut status, libc::WNOHANG) };
    assert_eq!(waited, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}
