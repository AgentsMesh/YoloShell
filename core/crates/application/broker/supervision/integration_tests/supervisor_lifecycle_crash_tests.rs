use super::*;
use crate::ChildLaunch;
use crate::supervision_activation::{self, BrokerReadiness};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use yolo_broker_supervision_state::{SupervisionPaths, io_kind};
use yolo_broker_supervision_state::{SupervisionPhase, SupervisionRecord};
use yolo_process::{DetachedChildCustody, ProcessIdentity};
use yolo_remote_broker::{BrokerLaunchDeadline, BrokerPaths};

const CONFIG_PATH: &str = "/tmp/yolo-supervision-crash-config";
const CONFIG_REVISION: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const BROKER_HOST: &str = "crash-window-host";
const EXIT_EXECUTABLE: &str = crate::test_executables::TRUE;
const HELPER_FILTER: &str = "supervisor_lifecycle::crash_tests::broker_crash_containment_helper";
const HELPER_ROLE_ENVIRONMENT: &str = "YOLO_BROKER_CRASH_TEST_ROLE";
const HELPER_ROLE: &str = "broker";
const HELPER_LIFETIME: std::time::Duration = std::time::Duration::from_secs(30);

#[test]
fn running_before_commit_crash_keeps_admission_closed_and_requires_containment_authority() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    let broker_paths = BrokerPaths::new(&state).unwrap();
    let deadline = Instant::now() + yolo_remote_broker::BROKER_READY_TIMEOUT;
    let options = options(state, &broker_paths, deadline);
    let paths = SupervisionPaths::for_launch(&options).unwrap();
    let supervisor = exited_identity();
    let launcher = ProcessIdentity::capture(std::process::id()).unwrap();
    let raw_child = Command::new(std::env::current_exe().unwrap())
        .args([
            crate::fixture_constants::TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_FILTER,
            crate::fixture_constants::TEST_HARNESS_NOCAPTURE_ARGUMENT,
        ])
        .env(HELPER_ROLE_ENVIRONMENT, HELPER_ROLE)
        .env(
            yolo_cli::constants::BROKER_SUPERVISION_AUTHORITY_ENVIRONMENT,
            options.authority().to_string(),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let broker = ProcessIdentity::capture(raw_child.id()).unwrap();
    let mut child = DetachedChildCustody::capture(raw_child).unwrap();
    let instance_id = uuid::Uuid::new_v4();
    let mut record = SupervisionRecord::intent(&options, supervisor, launcher).unwrap();
    record.activate(broker).unwrap();
    record
        .ready(&BrokerReadiness {
            instance_id,
            host_id: BROKER_HOST.into(),
            config_revision: CONFIG_REVISION.into(),
        })
        .unwrap();
    paths.write_record(&record).unwrap();
    paths.write_adoption(&record).unwrap();
    let (mut control, receiver) = supervision_activation::channel().unwrap();
    control.activate().unwrap();
    let signals = receiver
        .await_activation()
        .unwrap()
        .monitor_supervisor(deadline)
        .unwrap();
    let opened = Arc::new(AtomicBool::new(false));
    let opened_after_ack = Arc::clone(&opened);
    let adoption = std::thread::spawn(move || {
        let accepted = signals.adoption.blocking_recv().unwrap()(instance_id);
        opened_after_ack.store(accepted, Ordering::SeqCst);
        accepted
    });
    let _crash = test_hook::install();

    let crash = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        supervise(&mut child, &mut control, &mut record, &paths, deadline)
    }));
    assert!(crash.is_err());
    assert_eq!(
        paths.read_record().unwrap().phase(),
        SupervisionPhase::Running
    );
    drop(control);
    assert!(!adoption.join().unwrap());
    assert!(!opened.load(Ordering::SeqCst));

    settle_after_crash(&options, &paths, &mut child, broker);
    assert!(
        paths
            .read_record()
            .is_err_and(|error| io_kind(&error) == Some(std::io::ErrorKind::NotFound))
    );
}

fn settle_after_crash(
    options: &ChildLaunch,
    _paths: &SupervisionPaths,
    child: &mut DetachedChildCustody,
    _broker: ProcessIdentity,
) {
    #[cfg(target_os = "macos")]
    SupervisionPaths::for_launch(options).unwrap();
    #[cfg(target_os = "linux")]
    {
        let error = SupervisionPaths::for_launch(options).unwrap_err();
        assert!(
            error
                .downcast_ref::<yolo_process::DestructiveSessionUnavailable>()
                .is_some()
        );
        assert_eq!(
            _paths.read_record().unwrap().phase(),
            SupervisionPhase::Running
        );
        assert!(_broker.matches());
        child.request_direct_termination().unwrap();
    }
    child.wait().unwrap();
    SupervisionPaths::for_launch(options).unwrap();
}

#[test]
fn broker_crash_containment_helper() {
    if std::env::var(HELPER_ROLE_ENVIRONMENT).as_deref() == Ok(HELPER_ROLE) {
        std::thread::sleep(HELPER_LIFETIME);
    }
}

fn options(state: std::path::PathBuf, paths: &BrokerPaths, deadline: Instant) -> ChildLaunch {
    ChildLaunch::new(
        CONFIG_PATH.into(),
        state,
        CONFIG_REVISION.into(),
        paths.state_anchor(),
        uuid::Uuid::new_v4().to_string().parse().unwrap(),
        BrokerLaunchDeadline::capture(deadline.into()).unwrap(),
    )
    .unwrap()
}

fn exited_identity() -> ProcessIdentity {
    let mut child = Command::new(EXIT_EXECUTABLE).spawn().unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    child.wait().unwrap();
    identity
}
