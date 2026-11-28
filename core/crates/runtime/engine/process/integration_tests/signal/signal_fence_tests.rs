use super::ProcessIdentity;
use super::signal_hook::install_pre_signal_hook;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

const TEST_SLEEP_EXECUTABLE: &str = "/bin/sleep";
const TEST_SLEEP_DURATION: &str = "30";

#[test]
fn target_exit_at_the_final_signal_boundary_is_an_atomic_noop() {
    let mut original = spawn_sleep();
    let original_identity = ProcessIdentity::capture(original.id()).unwrap();
    let replacement = Rc::new(RefCell::new(None));
    let installed = Rc::clone(&replacement);
    let _hook = install_pre_signal_hook(move || {
        let _ = original.kill();
        let _ = original.wait();
        let child = spawn_sleep();
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        *installed.borrow_mut() = Some((child, identity));
    });

    assert!(!original_identity.signal(libc::SIGKILL).unwrap());
    let (mut child, identity) = replacement.borrow_mut().take().unwrap();
    let survived = identity.matches();
    let _ = identity.signal(libc::SIGKILL);
    let _ = child.wait();
    assert!(survived);
}

fn spawn_sleep() -> std::process::Child {
    crate::process::spawn_waitable_test_child(
        std::process::Command::new(Path::new(TEST_SLEEP_EXECUTABLE)).arg(TEST_SLEEP_DURATION),
    )
}
