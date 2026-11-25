use super::{
    CapturedProcessTree, MacosProcessCapabilityRelease, ProcessDiscoveryCapability,
    ProcessIdentity, macos_native,
};
use serde::Serialize;
use std::time::{Duration, Instant};

#[path = "captured_tree_discovery_fixture_macos.rs"]
mod fixture;
use fixture::RapidEscapeFixture;

const DISCOVERY_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(8);
const DISCOVERY_POLL_INTERVAL: Duration = Duration::from_millis(10);
const DISCOVERY_TIMEOUT_DETAIL: &str = "escaped discovery capability scan did not settle";
const EXPECTED_WATCHDOG_IDENTITIES: usize = 1;
const BAZEL_TIMEOUT_PROBE_PATH_ENVIRONMENT: &str = "YOLO_BAZEL_TIMEOUT_PROBE_PATH";
const BAZEL_TIMEOUT_PROBE_HOLD: Duration = Duration::from_secs(30);
const BAZEL_TIMEOUT_PROBE_DETAIL: &str = "Bazel timeout probe environment is unavailable";

#[derive(Serialize)]
struct BazelTimeoutProbe {
    guard: ProbeIdentity,
    test: ProbeIdentity,
    watchdog: ProbeIdentity,
    leaf: ProbeIdentity,
    socket_root: String,
}

#[derive(Serialize)]
struct ProbeIdentity {
    pid: u32,
    start_time: u64,
}

#[test]
fn capability_scan_captures_a_double_fork_escape_missed_by_lineage_polling() {
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let mut fixture = RapidEscapeFixture::spawn_with_scrubbed_leaf_capability(&capability);
    let root = fixture.root_identity();
    let root_kernel = macos_native::kernel_process_metadata(root.pid).unwrap();
    let mut tree = capability.capture_process_tree(root).unwrap();
    assert_eq!(tree.identities(), [root]);

    let leaf = fixture.release_and_capture_leaf();
    let leaf_kernel = macos_native::kernel_process_metadata(leaf.pid).unwrap();
    assert_ne!(leaf_kernel.parent_pid, root.pid);
    assert_ne!(leaf_kernel.parent_unique_id, root_kernel.unique_id);
    assert!(!tree.identities().contains(&leaf));

    tree.begin_discovery_cleanup_internal();
    let deadline = Instant::now() + DISCOVERY_SETTLEMENT_TIMEOUT;
    while !tree.refresh().unwrap().contains(&leaf) {
        assert!(Instant::now() < deadline, "{DISCOVERY_TIMEOUT_DETAIL}");
        std::thread::sleep(DISCOVERY_POLL_INTERVAL);
    }
    assert!(
        tree.terminate_captured(libc::SIGKILL)
            .unwrap()
            .contains(&leaf)
    );
    while !tree.discovery_is_settled_internal() || !tree.live_identities().unwrap().is_empty() {
        assert!(Instant::now() < deadline, "{DISCOVERY_TIMEOUT_DETAIL}");
        tree.refresh().unwrap();
        tree.terminate_captured(libc::SIGKILL).unwrap();
        std::thread::sleep(DISCOVERY_POLL_INTERVAL);
    }
    fixture.wait_for_leaf_exit();
}

#[test]
fn writer_present_scan_finds_a_reparented_exact_holder() {
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let authority = capability.descriptor_authority();
    let mut fixture = RapidEscapeFixture::spawn_with_scrubbed_leaf_capability(&capability);
    let root = fixture.root_identity();
    let root_kernel = macos_native::kernel_process_metadata(root.pid).unwrap();
    let mut tree = capability.capture_process_tree(root).unwrap();

    let leaf = fixture.release_and_capture_leaf();
    let leaf_kernel = macos_native::kernel_process_metadata(leaf.pid).unwrap();
    assert_ne!(leaf_kernel.parent_pid, root.pid);
    assert_ne!(leaf_kernel.parent_unique_id, root_kernel.unique_id);
    assert_eq!(
        authority.poll_release().unwrap(),
        MacosProcessCapabilityRelease::WriterPresent
    );

    tree.begin_discovery_cleanup_internal();
    let deadline = Instant::now() + DISCOVERY_SETTLEMENT_TIMEOUT;
    while !tree.refresh().unwrap().contains(&leaf) {
        assert!(Instant::now() < deadline, "{DISCOVERY_TIMEOUT_DETAIL}");
        std::thread::sleep(DISCOVERY_POLL_INTERVAL);
    }
    assert!(tree.identities().contains(&leaf));
    assert!(!tree.discovery_is_authoritatively_empty_internal());
}

#[test]
fn bazel_timeout_probe_holds_a_reparented_discovery_leaf() {
    let Some(report_path) = std::env::var_os(BAZEL_TIMEOUT_PROBE_PATH_ENVIRONMENT) else {
        return;
    };
    let capability = std::env::var(crate::process::PROCESS_DISCOVERY_CAPABILITY_ENV)
        .expect(BAZEL_TIMEOUT_PROBE_DETAIL);
    let mut fixture = RapidEscapeFixture::spawn(&capability);
    let leaf = fixture.release_and_capture_leaf();
    let test = ProcessIdentity::capture(std::process::id()).unwrap();
    let parent = unsafe { libc::getppid() };
    let guard = ProcessIdentity::capture(u32::try_from(parent).unwrap()).unwrap();
    let watchdog = watchdog_identity(guard, test);
    let socket_root =
        std::env::var_os(yolo_paths::SOCKET_ROOT_ENVIRONMENT).expect(BAZEL_TIMEOUT_PROBE_DETAIL);
    let report = BazelTimeoutProbe {
        guard: guard.into(),
        test: test.into(),
        watchdog: watchdog.into(),
        leaf: leaf.into(),
        socket_root: socket_root.to_string_lossy().into_owned(),
    };
    std::fs::write(report_path, serde_json::to_vec(&report).unwrap()).unwrap();
    std::thread::sleep(BAZEL_TIMEOUT_PROBE_HOLD);
}

fn watchdog_identity(guard: ProcessIdentity, test: ProcessIdentity) -> ProcessIdentity {
    let tree = CapturedProcessTree::capture(guard).unwrap();
    let leaders = tree
        .identities()
        .into_iter()
        .filter(|identity| *identity != guard && *identity != test)
        .filter(|identity| exact_process_group(*identity) == Some(identity.pid))
        .filter(|identity| exact_session(*identity) == Some(identity.pid))
        .collect::<Vec<_>>();
    assert_eq!(leaders.len(), EXPECTED_WATCHDOG_IDENTITIES);
    leaders[0]
}

fn exact_process_group(identity: ProcessIdentity) -> Option<u32> {
    exact_process_value(identity, |pid| unsafe { libc::getpgid(pid) })
}

fn exact_session(identity: ProcessIdentity) -> Option<u32> {
    exact_process_value(identity, |pid| unsafe { libc::getsid(pid) })
}

fn exact_process_value(
    identity: ProcessIdentity,
    read: impl FnOnce(libc::pid_t) -> libc::pid_t,
) -> Option<u32> {
    if !identity.matches() {
        return None;
    }
    let pid = libc::pid_t::try_from(identity.pid).ok()?;
    let value = read(pid);
    (value > libc::pid_t::default() && identity.matches()).then(|| u32::try_from(value).unwrap())
}

impl From<ProcessIdentity> for ProbeIdentity {
    fn from(identity: ProcessIdentity) -> Self {
        Self {
            pid: identity.pid,
            start_time: identity.start_time,
        }
    }
}
