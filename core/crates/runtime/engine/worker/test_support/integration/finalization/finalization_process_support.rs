const FINALIZER_PROCESS_SCENARIO_ENVIRONMENT: &str = "YOLO_TEST_FINALIZER_SCENARIO";
const FINALIZER_PROCESS_START_GATE_ENVIRONMENT: &str = "YOLO_TEST_FINALIZER_START_GATE";
const FINALIZER_PROCESS_ACKNOWLEDGEMENT_ENVIRONMENT: &str = "YOLO_TEST_FINALIZER_ACKNOWLEDGEMENT";
const FINALIZER_RESOURCE_SCENARIO: &str = "resource-mutations";
const FINALIZER_CONTROLLER_SCENARIO: &str = "controller-fencing";
const FINALIZER_AUTHORITY_SCENARIO: &str = "authority-upgrade";
const FINALIZER_PROCESS_START_GATE_FILE: &str = "child-gate";
const FINALIZER_PROCESS_ACKNOWLEDGEMENT_FILE: &str = "acknowledged";
const FINALIZER_PROCESS_START_GATE_MARKER: &[u8] = b"";
const FINALIZER_RESOURCE_TEST: &str =
    "worker::tests::finalization_barrier::finalizer_resource_mutations_wait_for_admitted_input";
const FINALIZER_CONTROLLER_TEST: &str = "worker::tests::finalization_barrier::pending_controller_finalizer_fences_adoption_and_commits_shutdown";
const FINALIZER_AUTHORITY_TEST: &str = "worker::tests::finalization_controller_authority::ownership_loss_upgrades_reason_and_authority";
const FINALIZER_PROCESS_TIMEOUT: Duration = Duration::from_secs(10);
const FINALIZER_PROCESS_REAP_TIMEOUT: Duration = CONTROL_BARRIER_TIMEOUT;
const FINALIZER_PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(5);
const FINALIZER_PRIVATE_PROCESS_GROUP: i32 = 0;
const FINALIZER_PROCESS_GROUP_ERROR: libc::pid_t = -1;
const FINALIZER_PROCESS_EXECUTABLE_DETAIL: &str = "resolve finalizer test executable";
const FINALIZER_PROCESS_SPAWN_DETAIL: &str = "spawn isolated finalizer test";
const FINALIZER_PROCESS_IDENTITY_DETAIL: &str = "capture isolated finalizer process identity";
const FINALIZER_PROCESS_GROUP_DETAIL: &str =
    "isolated finalizer test did not enter its private process group";
const FINALIZER_PROCESS_STATUS_DETAIL: &str = "isolated finalizer test failed";
const FINALIZER_PROCESS_TIMEOUT_DETAIL: &str = "isolated finalizer test exceeded its deadline";
const FINALIZER_PROCESS_REAP_DETAIL: &str =
    "isolated finalizer process tree did not settle after termination";
const FINALIZER_PROCESS_WAIT_DETAIL: &str = "observe isolated finalizer test status";
const FINALIZER_PROCESS_SCENARIO_DETAIL: &str = "isolated finalizer scenario does not match";
const FINALIZER_PROCESS_START_GATE_DETAIL: &str =
    "isolated finalizer child did not receive its start gate";
const FINALIZER_PROCESS_START_TIMEOUT_DETAIL: &str =
    "isolated finalizer child start gate exceeded its deadline";
const FINALIZER_PROCESS_START_RELEASE_DETAIL: &str = "release isolated finalizer child start gate";
const FINALIZER_PROCESS_ACKNOWLEDGEMENT_DETAIL: &str =
    "isolated finalizer child did not acknowledge its exact test";

include!("finalization_process_scenarios.rs");

fn wait_for_finalizer_start_gate() {
    let gate = std::env::var_os(FINALIZER_PROCESS_START_GATE_ENVIRONMENT)
        .expect(FINALIZER_PROCESS_START_GATE_DETAIL);
    let deadline = Instant::now() + FINALIZER_PROCESS_TIMEOUT;
    while !std::path::Path::new(&gate).exists() {
        assert!(
            Instant::now() < deadline,
            "{FINALIZER_PROCESS_START_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(FINALIZER_PROCESS_POLL_INTERVAL);
    }
}

fn acknowledge_finalizer_child() {
    let path = std::env::var_os(FINALIZER_PROCESS_ACKNOWLEDGEMENT_ENVIRONMENT)
        .expect(FINALIZER_PROCESS_ACKNOWLEDGEMENT_DETAIL);
    std::fs::write(path, FINALIZER_PROCESS_START_GATE_MARKER)
        .expect(FINALIZER_PROCESS_ACKNOWLEDGEMENT_DETAIL);
}

struct FinalizerTestProcess {
    child: Option<crate::process::DetachedChildCustody>,
    identity: Option<crate::process::ProcessIdentity>,
    containment: Option<crate::process::CapturedProcessTree>,
    acknowledgement: std::path::PathBuf,
    settled: bool,
}

impl FinalizerTestProcess {
    fn run(scenario: FinalizerProcessScenario) {
        use std::os::unix::process::CommandExt as _;

        let deadline = Instant::now() + FINALIZER_PROCESS_TIMEOUT;
        let executable = std::env::current_exe().expect(FINALIZER_PROCESS_EXECUTABLE_DETAIL);
        let start_gate_directory = tempfile::tempdir().expect(FINALIZER_PROCESS_START_GATE_DETAIL);
        let start_gate = start_gate_directory
            .path()
            .join(FINALIZER_PROCESS_START_GATE_FILE);
        let acknowledgement = start_gate_directory
            .path()
            .join(FINALIZER_PROCESS_ACKNOWLEDGEMENT_FILE);
        let mut command = std::process::Command::new(executable);
        command
            .args([
                crate::constants::TEST_HARNESS_EXACT_ARGUMENT,
                scenario.test_name(),
                crate::constants::TEST_HARNESS_NOCAPTURE_ARGUMENT,
                crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
            ])
            .env(
                FINALIZER_PROCESS_SCENARIO_ENVIRONMENT,
                scenario.environment_value(),
            )
            .env(FINALIZER_PROCESS_START_GATE_ENVIRONMENT, &start_gate)
            .env(
                FINALIZER_PROCESS_ACKNOWLEDGEMENT_ENVIRONMENT,
                &acknowledgement,
            );
        command.process_group(FINALIZER_PRIVATE_PROCESS_GROUP);
        let spawn_guard = crate::process::waitable_child_spawn_guard_before(deadline)
            .expect(FINALIZER_PROCESS_SPAWN_DETAIL);
        let child =
            crate::process::spawn_waitable_test_child_in_boundary(command, &spawn_guard.boundary());
        drop(spawn_guard);
        let child = crate::process::DetachedChildCustody::capture(child)
            .unwrap_or_else(crate::process::DetachedChildCustodyError::into_custody);
        let mut process = Self {
            child: Some(child),
            identity: None,
            containment: None,
            acknowledgement,
            settled: false,
        };
        process.identity = process.child.as_ref().and_then(|child| child.identity());
        let identity = process.identity.expect(FINALIZER_PROCESS_IDENTITY_DETAIL);
        process.verify_private_group(identity);
        process.containment = Some(
            crate::process::CapturedProcessTree::capture(identity)
                .expect(FINALIZER_PROCESS_IDENTITY_DETAIL),
        );
        std::fs::write(&start_gate, FINALIZER_PROCESS_START_GATE_MARKER)
            .expect(FINALIZER_PROCESS_START_RELEASE_DETAIL);
        process.finish(deadline);
    }

    fn finish(mut self, deadline: Instant) {
        loop {
            let _ = self.refresh_containment();
            let status = self
                .child
                .as_mut()
                .expect(FINALIZER_PROCESS_WAIT_DETAIL)
                .try_wait()
                .expect(FINALIZER_PROCESS_WAIT_DETAIL);
            if let Some(status) = status {
                self.child.take();
                if status.success() && self.acknowledgement.exists() && self.settle_naturally() {
                    self.settled = true;
                    return;
                }
                self.fail(&format!("{FINALIZER_PROCESS_STATUS_DETAIL}: {status}"));
            }
            if Instant::now() >= deadline {
                self.fail(FINALIZER_PROCESS_TIMEOUT_DETAIL);
            }
            std::thread::sleep(FINALIZER_PROCESS_POLL_INTERVAL);
        }
    }

    fn verify_private_group(&self, identity: crate::process::ProcessIdentity) {
        let pid = libc::pid_t::try_from(identity.pid).expect(FINALIZER_PROCESS_GROUP_DETAIL);
        let group = unsafe { libc::getpgid(pid) };
        assert_ne!(
            group, FINALIZER_PROCESS_GROUP_ERROR,
            "{FINALIZER_PROCESS_GROUP_DETAIL}"
        );
        assert_eq!(group, pid, "{FINALIZER_PROCESS_GROUP_DETAIL}");
    }
}
