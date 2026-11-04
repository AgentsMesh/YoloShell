use super::*;
use yolo_process_test_support::{ProcessIdentity, ProcessIdentityObservation};

const GUARDIAN_PAUSE_TIMEOUT: &str = "fixture guardian did not enter its stopped state";
const GUARDIAN_RESUME_FAILURE: &str = "fixture guardian resume could not be settled";

pub(super) struct PausedGuardian(Option<ProcessIdentity>);

impl PausedGuardian {
    pub(super) async fn capture(harness: &Harness, record: &RuntimeRecord) -> Self {
        #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
        {
            let records = harness.bind_cleanup_records(std::slice::from_ref(record));
            let guardians = guardians::capture(
                &harness.paths(),
                &records,
                GuardianExpectation::RequiredOnSupportedPlatform,
            )
            .unwrap();
            let guardian = guardians.into_iter().next().unwrap();
            guardian.confirm_active(&harness.paths()).unwrap();
            let identity = guardian.identity();
            let paused = Self(Some(identity));
            assert!(identity.signal(libc::SIGSTOP).unwrap());
            let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
            while !identity.is_stably_stopped().unwrap() {
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "{GUARDIAN_PAUSE_TIMEOUT}"
                );
                tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
            }
            guardian.confirm_active(&harness.paths()).unwrap();
            paused
        }
        #[cfg(not(all(target_os = "macos", feature = "adversarial-test-hooks")))]
        {
            let _ = (harness, record);
            Self(None)
        }
    }
}

impl Drop for PausedGuardian {
    fn drop(&mut self) {
        let Some(identity) = self.0.take() else {
            return;
        };
        let result = identity.signal(libc::SIGCONT).and_then(|delivered| {
            anyhow::ensure!(
                delivered || identity.observe() == ProcessIdentityObservation::Gone,
                GUARDIAN_RESUME_FAILURE
            );
            Ok(())
        });
        if let Err(error) = result {
            if std::thread::panicking() {
                eprintln!("{GUARDIAN_RESUME_FAILURE}: {error:#}");
            } else {
                panic!("{GUARDIAN_RESUME_FAILURE}: {error:#}");
            }
        }
    }
}
