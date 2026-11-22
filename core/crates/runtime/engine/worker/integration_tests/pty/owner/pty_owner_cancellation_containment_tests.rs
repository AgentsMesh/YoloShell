const OWNER_CANCELLATION_CONTAINMENT_ROLE_ENV: &str = "YOLO_OWNER_CANCELLATION_CONTAINMENT_ROLE";
const OWNER_CANCELLATION_CONTAINMENT_EVIDENCE_ENV: &str =
    "YOLO_OWNER_CANCELLATION_CONTAINMENT_EVIDENCE";
const OWNER_CANCELLATION_CONTAINMENT_WORKER_ROLE: &str = "worker";
const OWNER_CANCELLATION_CONTAINMENT_LEAF_ROLE: &str = "leaf";
const OWNER_CANCELLATION_CONTAINMENT_EVIDENCE_FILE: &str = "containment.json";
const OWNER_CANCELLATION_CONTAINMENT_HELPER_TEST: &str =
    "worker::tests::pty_owner_cancellation::owner_cancellation_prepublication_group_helper";
const OWNER_CANCELLATION_CONTAINMENT_PANIC: &str =
    "injected owner cancellation publication failure";
const OWNER_CANCELLATION_CONTAINMENT_WAIT_DETAIL: &str =
    "owner cancellation containment helper did not settle";

#[derive(Clone, Copy, Deserialize, Serialize)]
struct OwnerCancellationContainmentEvidence {
    worker_pid: u32,
    worker_start_time: u64,
    leaf_pid: u32,
    leaf_start_time: u64,
}

impl OwnerCancellationContainmentEvidence {
    fn worker(self) -> process::ProcessIdentity {
        process::ProcessIdentity {
            pid: self.worker_pid,
            start_time: self.worker_start_time,
        }
    }

    fn leaf(self) -> process::ProcessIdentity {
        process::ProcessIdentity {
            pid: self.leaf_pid,
            start_time: self.leaf_start_time,
        }
    }
}

#[test]
fn owner_cancellation_unwind_before_identity_publication_contains_private_group() {
    let directory = tempfile::tempdir().unwrap();
    let evidence_path = directory
        .path()
        .join(OWNER_CANCELLATION_CONTAINMENT_EVIDENCE_FILE);
    let command = owner_cancellation_containment_command(
        OWNER_CANCELLATION_CONTAINMENT_WORKER_ROLE,
        &evidence_path,
    );
    let observed = std::cell::Cell::new(None);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _child = OwnerCancellationChild::spawn(command).unwrap();
        observed.set(Some(wait_owner_cancellation_containment_evidence(
            &evidence_path,
        )));
        std::panic::panic_any(OWNER_CANCELLATION_CONTAINMENT_PANIC);
    }));
    let failure = outcome.unwrap_err();
    assert_eq!(
        failure.downcast_ref::<&str>().copied(),
        Some(OWNER_CANCELLATION_CONTAINMENT_PANIC)
    );
    let evidence = observed.get().unwrap();
    wait_owner_cancellation_identity_gone(evidence.worker());
    wait_owner_cancellation_identity_gone(evidence.leaf());
}

#[test]
fn owner_cancellation_prepublication_group_helper() {
    match std::env::var(OWNER_CANCELLATION_CONTAINMENT_ROLE_ENV).as_deref() {
        Ok(OWNER_CANCELLATION_CONTAINMENT_WORKER_ROLE) => {
            run_owner_cancellation_containment_worker()
        }
        Ok(OWNER_CANCELLATION_CONTAINMENT_LEAF_ROLE) => {
            wait_owner_cancellation_containment_lifetime()
        }
        _ => {}
    }
}

fn run_owner_cancellation_containment_worker() {
    let evidence_path = std::env::var_os(OWNER_CANCELLATION_CONTAINMENT_EVIDENCE_ENV)
        .map(PathBuf::from)
        .unwrap();
    let mut command = owner_cancellation_containment_command(
        OWNER_CANCELLATION_CONTAINMENT_LEAF_ROLE,
        &evidence_path,
    );
    let leaf = process::spawn_waitable_test_child(&mut command);
    let worker = process::ProcessIdentity::capture(std::process::id()).unwrap();
    let leaf_identity = process::ProcessIdentity::capture(leaf.id()).unwrap();
    let evidence = OwnerCancellationContainmentEvidence {
        worker_pid: worker.pid,
        worker_start_time: worker.start_time,
        leaf_pid: leaf_identity.pid,
        leaf_start_time: leaf_identity.start_time,
    };
    atomic_write_json(&evidence_path, &evidence).unwrap();
    wait_owner_cancellation_containment_lifetime();
}

fn owner_cancellation_containment_command(
    role: &str,
    evidence_path: &Path,
) -> std::process::Command {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            BOOTSTRAP_TEST_EXACT_ARGUMENT,
            OWNER_CANCELLATION_CONTAINMENT_HELPER_TEST,
            BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
            crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(OWNER_CANCELLATION_CONTAINMENT_ROLE_ENV, role)
        .env(OWNER_CANCELLATION_CONTAINMENT_EVIDENCE_ENV, evidence_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    command
}

fn wait_owner_cancellation_containment_evidence(
    path: &Path,
) -> OwnerCancellationContainmentEvidence {
    let deadline = Instant::now() + OWNER_CANCELLATION_SETUP_TIMEOUT;
    loop {
        if let Ok(evidence) = read_json(path) {
            return evidence;
        }
        assert!(
            Instant::now() < deadline,
            "{OWNER_CANCELLATION_CONTAINMENT_WAIT_DETAIL}"
        );
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

fn wait_owner_cancellation_identity_gone(identity: process::ProcessIdentity) {
    let deadline = Instant::now() + OWNER_CANCELLATION_CHILD_REAP_TIMEOUT;
    loop {
        if identity.observe() == process::ProcessIdentityObservation::Gone {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{OWNER_CANCELLATION_CONTAINMENT_WAIT_DETAIL}: {identity:?}"
        );
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

fn wait_owner_cancellation_containment_lifetime() -> ! {
    loop {
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}
