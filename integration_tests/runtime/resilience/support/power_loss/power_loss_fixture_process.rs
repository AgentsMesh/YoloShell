use super::*;

pub(super) struct TestProcess {
    pub(super) identity: ProcessIdentity,
    reaper: Option<std::thread::JoinHandle<()>>,
    #[cfg(target_os = "linux")]
    pty: portable_pty::PtyPair,
    #[cfg(target_os = "linux")]
    capability: String,
    #[cfg(target_os = "linux")]
    containment: std::cell::RefCell<
        Option<yolo_process_test_support::linux_containment::LinuxContainmentLease>,
    >,
}

impl TestProcess {
    pub(super) fn is_stably_stopped(&self) -> bool {
        yolo_process_test_support::ProcessIdentity {
            pid: self.identity.pid,
            start_time: self.identity.start_time,
        }
        .is_stably_stopped()
        .unwrap()
    }

    #[cfg(not(target_os = "linux"))]
    pub(super) fn spawn() -> Self {
        let mut child = Command::new(TEST_PROCESS_EXECUTABLE)
            .arg(TEST_PROCESS_ARGUMENT)
            .spawn()
            .unwrap();
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let reaper = std::thread::spawn(move || {
            let _ = child.wait();
        });
        Self {
            identity,
            reaper: Some(reaper),
        }
    }

    #[cfg(target_os = "linux")]
    pub(super) fn spawn() -> Self {
        let pty = portable_pty::native_pty_system()
            .openpty(portable_pty::PtySize::default())
            .unwrap();
        let capability = Uuid::new_v4().to_string();
        let mut command = portable_pty::CommandBuilder::new(TEST_PROCESS_EXECUTABLE);
        command.arg(TEST_PROCESS_ARGUMENT);
        command.env(
            yolo_process_test_support::PTY_SESSION_CAPABILITY_ENV,
            &capability,
        );
        let mut child = pty.slave.spawn_command(command).unwrap();
        let identity = ProcessIdentity::capture(child.process_id().unwrap()).unwrap();
        let reaper = std::thread::spawn(move || {
            let _ = child.wait();
        });
        Self {
            identity,
            reaper: Some(reaper),
            pty,
            capability,
            containment: Default::default(),
        }
    }

    #[cfg(target_os = "linux")]
    pub(super) fn bind_containment(&self, paths: &Paths, runtime: &RuntimeRecord) {
        use yolo_process_test_support::linux_containment::{LinuxContainmentBroker, record_path};
        let broker = LinuxContainmentBroker::discover()
            .unwrap()
            .expect("private fixture cgroup delegation");
        let mut lease = broker
            .prepare(runtime.terminal_id, runtime.runtime_epoch)
            .unwrap();
        let proof = yolo_process_test_support::PtySessionProof::capture(
            self.pty.master.tty_name().unwrap(),
            self.capability.clone(),
        )
        .unwrap();
        lease.bind_pty_proof(&proof).unwrap();
        lease
            .migrate_bootstrap(yolo_process_test_support::ProcessIdentity {
                pid: self.identity.pid,
                start_time: self.identity.start_time,
            })
            .unwrap();
        let path = record_path(
            &paths.session_dir(runtime.terminal_id),
            runtime.runtime_epoch,
        );
        yolo_runtime::paths::atomic_write_json(&path, lease.record()).unwrap();
        *self.containment.borrow_mut() = Some(lease);
    }
}

impl Drop for TestProcess {
    fn drop(&mut self) {
        let _ = self.identity.signal(libc::SIGKILL);
        if let Some(reaper) = self.reaper.take() {
            reaper.join().unwrap();
        }
        #[cfg(target_os = "linux")]
        if let Some(lease) = self.containment.get_mut().take() {
            let _ = lease.remove_empty();
        }
    }
}
