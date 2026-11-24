const SHELL_PATH: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const DESCENDANT_SCRIPT: &str = "trap '' TERM; /bin/sleep 30 & /bin/sleep 30 & wait";
const MATCH_FAILURE_DETAIL: &str = "injected exact process match failure";
const REFRESH_FAILURE_DETAIL: &str = "injected cleanup refresh failure";
const SIGNAL_FAILURE_DETAIL: &str = "injected partial captured-target signal failure";
const PRIOR_CLEANUP_FAILURE_DETAIL: &str = "injected failure before final containment timeout";
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(2);
const CAPTURE_POLL_INTERVAL: Duration = Duration::from_millis(5);
const DESCENDANT_CAPTURE_MISSING_DETAIL: &str = "descendant was not captured";
const ROOT_AND_DESCENDANT_COUNT: usize = 3;
const FIRST_SIGNAL_TARGET_INDEX: usize = 0;
const ZERO_TIMEOUT_SCHEDULE: CleanupSchedule = CleanupSchedule {
    graceful_timeout: Duration::ZERO,
    force_timeout: Duration::ZERO,
    force_confirmation_timeout: Duration::ZERO,
};

struct TreeFixture {
    root: Child,
    tree: CapturedProcessTree,
}

impl TreeFixture {
    fn spawn() -> Self {
        let root = crate::process::spawn_waitable_test_child(
            Command::new(SHELL_PATH)
                .args([SHELL_COMMAND_ARGUMENT, DESCENDANT_SCRIPT])
                .stdout(Stdio::null())
                .stderr(Stdio::null()),
        );
        let identity = ProcessIdentity::capture(root.id()).unwrap();
        let mut tree = CapturedProcessTree::capture(identity).unwrap();
        let deadline = Instant::now() + CAPTURE_TIMEOUT;
        while tree.identities().len() < ROOT_AND_DESCENDANT_COUNT {
            assert!(
                Instant::now() < deadline,
                "{}",
                DESCENDANT_CAPTURE_MISSING_DETAIL
            );
            std::thread::sleep(CAPTURE_POLL_INTERVAL);
            tree.refresh().unwrap();
        }
        Self { root, tree }
    }

    fn descendants(&self) -> Vec<ProcessIdentity> {
        self.tree
            .identities()
            .into_iter()
            .filter(|identity| identity.pid != self.root.id())
            .collect()
    }
}

impl Drop for TreeFixture {
    fn drop(&mut self) {
        let _ = self.tree.terminate(libc::SIGKILL);
        let _ = self.root.wait();
    }
}
