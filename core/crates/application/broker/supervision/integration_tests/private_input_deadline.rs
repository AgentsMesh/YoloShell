use crate::fixture_constants as fixture;
use crate::{ChildLaunch, CredentialSource, DetachedSpawner, PendingBroker};
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

const MAX_TEST_IDENTITIES: usize = 32;
const MAX_TEST_PASSPHRASE_BYTES: usize = 4 * 1_024;
const PASSPHRASE_CHARACTER: char = '\u{0001}';
const NONREADING_SCRIPT_FILE: &str = "nonreading-child.sh";
const NONREADING_PID_FILE: &str = "nonreading-child.pid";
const DELIVERY_BUDGET: Duration = Duration::from_secs(3);
const RETURN_BOUND: Duration = Duration::from_secs(5);
const REAP_POLL: Duration = Duration::from_millis(5);

#[test]
fn maximum_valid_private_input_has_bounded_delivery_and_exact_custody() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join(NONREADING_SCRIPT_FILE);
    let pid_file = root.path().join(NONREADING_PID_FILE);
    std::fs::write(
        &executable,
        format!(
            "#!/bin/sh\nprintf '%s' \"$$\" > '{}'\nwhile :; do :; done\n",
            pid_file.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(
        &executable,
        std::fs::Permissions::from_mode(fixture::OWNER_EXECUTABLE_MODE),
    )
    .unwrap();
    let passphrases = maximum_passphrases();
    let launch = options(root.path());
    let started = Instant::now();

    let error = DetachedSpawner::new(executable, passphrases, CredentialSource::PrivateStdin)
        .spawn(&launch, None)
        .err()
        .unwrap();

    assert!(started.elapsed() < RETURN_BOUND);
    assert!(format!("{error:#}").contains("deadline"));
    let process_id = std::fs::read_to_string(pid_file)
        .unwrap()
        .parse::<u32>()
        .unwrap();
    wait_until_reaped(process_id);
}

#[test]
fn child_returned_after_deadline_is_compensated_without_foreground_wait() {
    let child = std::process::Command::new(fixture::TEST_SLEEP_EXECUTABLE)
        .arg(fixture::TEST_SLEEP_DURATION)
        .spawn()
        .unwrap();
    let process_id = child.id();
    let spawned = PendingBroker::detached(
        yolo_process::DetachedChildCustody::capture(child).unwrap(),
        yolo_broker_child_custody::reserve_termination_custody().unwrap(),
    );

    let error = crate::start_detached::finish_spawn(spawned, Instant::now())
        .err()
        .unwrap();

    assert!(error.to_string().contains("deadline expired"));
    wait_until_reaped(process_id);
}

fn maximum_passphrases() -> yolo_remote_broker::IdentityPassphrases {
    let secret = PASSPHRASE_CHARACTER
        .to_string()
        .repeat(MAX_TEST_PASSPHRASE_BYTES);
    let encoded = yolo_capnp_codec::encode(&serde_json::json!({
        (fixture::VERSION_FIELD): fixture::TEST_PASSPHRASE_WIRE_VERSION,
        (fixture::BINDING_NONCE_FIELD): fixture::TEST_BINDING_NONCE,
        (fixture::HOST_ID_FIELD): fixture::TEST_HOST_ID,
        (fixture::IDENTITY_CREDENTIAL_REVISION_FIELD): null,
        (fixture::IDENTITY_PATHS_SHA256_FIELD): fixture::TEST_IDENTITY_PATHS_SHA256,
        (fixture::PASSPHRASES_FIELD): vec![secret; MAX_TEST_IDENTITIES],
    }))
    .unwrap();
    yolo_remote_broker::IdentityPassphrases::read_checked(encoded.as_slice(), false).unwrap()
}

fn options(root: &std::path::Path) -> ChildLaunch {
    let state_dir = root.join(fixture::STATE_DIRECTORY);
    let paths = yolo_remote_broker::BrokerPaths::new(&state_dir).unwrap();
    let deadline = yolo_remote_broker::BrokerLaunchDeadline::capture(
        tokio::time::Instant::now() + DELIVERY_BUDGET,
    )
    .unwrap();
    ChildLaunch::new(
        root.join(fixture::CONFIG_FILE),
        state_dir,
        fixture::TEST_CONFIG_REVISION.into(),
        paths.state_anchor(),
        uuid::Uuid::nil().to_string().parse().unwrap(),
        deadline,
    )
    .unwrap()
}

fn wait_until_reaped(process_id: u32) {
    let deadline = Instant::now() + RETURN_BOUND;
    while wait_authority_exists(process_id) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(REAP_POLL);
    }
}

fn wait_authority_exists(process_id: u32) -> bool {
    let mut status = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            process_id as libc::id_t,
            status.as_mut_ptr(),
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if result == libc::c_int::default() {
        return true;
    }
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
    false
}
