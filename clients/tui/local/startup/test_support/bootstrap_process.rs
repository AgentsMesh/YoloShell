use super::{bootstrap_contract::*, installation::Installation};
use std::{fs, path::Path};
use tokio::time::{Instant, timeout_at};
use yolo_local_profile::LocalProfile;
use yolo_process::{ProcessIdentity, ProcessIdentityObservation};

pub fn fixture(mode: &str) -> (Installation, LocalProfile) {
    let installation = Installation::new();
    fs::copy(
        std::env::var_os(BINARY_ENV).unwrap(),
        &installation.executable,
    )
    .unwrap();
    let mut profile = LocalProfile::from_application_support(installation.root.path()).unwrap();
    profile.state = installation.root.path().join(mode);
    profile.host_home = installation.root.path().join(HOST_HOME);
    (installation, profile)
}

pub async fn wait_ready(state: &Path, deadline: Instant) -> ProcessIdentity {
    timeout_at(deadline, async {
        loop {
            if let Ok(value) = fs::read_to_string(state.join(PID_FILE)) {
                if let Ok(pid) = value.parse() {
                    let identity = ProcessIdentity::capture(pid).unwrap();
                    assert_eq!(identity.observe(), ProcessIdentityObservation::Matching);
                    return identity;
                }
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}

pub async fn settle_aborted_helper(identity: ProcessIdentity, deadline: Instant) {
    yolo_process::wait_for_detached_termination_delivery(deadline.into_std()).unwrap();
    timeout_at(deadline, async {
        while identity.observe() != ProcessIdentityObservation::Gone {
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    // Only the deliberately cancelled/oversized fixture uses this path. Its
    // retained identity is proven gone; normal completions keep their profiles.
    yolo_coverage_test_support::discard_aborted_process_profile(identity.pid).unwrap();
}
