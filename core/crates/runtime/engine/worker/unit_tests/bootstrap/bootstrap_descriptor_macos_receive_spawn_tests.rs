use super::*;
use std::process::{Child, Command};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::Duration;

const SPAWN_PROBE_TIMEOUT: Duration = Duration::from_millis(25);
const SPAWN_RELEASE_TIMEOUT: Duration = Duration::from_secs(2);
const SPAWN_PROBE_EXECUTABLE: &str = "/bin/sleep";
const SPAWN_PROBE_ARGUMENT: &str = "5";
const FCNTL_FAILURE: libc::c_int = -1;

struct ProbeChild(Child);

impl Drop for ProbeChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn recvmsg_to_cloexec_transition_excludes_concurrent_guarded_spawn() {
    let (manager, child) = std::os::unix::net::UnixStream::pair().unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let writer_handle = issuer.identity().writer_handle();
    send_once(
        manager.as_raw_fd(),
        issuer.writer_descriptor(),
        crate::process::MACOS_PROCESS_CAPABILITY_TRANSFER_MARKER,
    )
    .unwrap();
    let pause = receive_descriptor::receive_test_hook::install(child.as_raw_fd());
    let receive_thread = std::thread::spawn(move || receive_once(child.as_raw_fd()));
    pause.wait_until_reached();

    let (spawned, observation) = channel();
    let spawn_thread = std::thread::spawn(move || {
        let _guard = crate::process::waitable_child_spawn_guard().unwrap();
        let child = Command::new(SPAWN_PROBE_EXECUTABLE)
            .arg(SPAWN_PROBE_ARGUMENT)
            .spawn()
            .unwrap();
        spawned.send(child).unwrap();
    });
    assert!(matches!(
        observation.recv_timeout(SPAWN_PROBE_TIMEOUT),
        Err(RecvTimeoutError::Timeout)
    ));

    pause.release();
    let writer = receive_thread.join().unwrap().unwrap();
    let flags = unsafe { libc::fcntl(writer.descriptor(), libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
    let child = ProbeChild(observation.recv_timeout(SPAWN_RELEASE_TIMEOUT).unwrap());
    spawn_thread.join().unwrap();
    assert!(
        !crate::process::process_holds_capability_for_test(child.0.id(), writer_handle).unwrap()
    );
    drop(writer);
}
