use super::*;
use crate::ChildLaunch;
use std::process::{Command, Stdio};
use yolo_broker_child_custody::reserve_termination_custody;
use yolo_process::{DetachedChildCustody, ProcessIdentity};
use yolo_remote_broker::{BrokerLaunchDeadline, BrokerPaths};

const PRIMARY_FAILURE: &str = "injected activation failure";
use crate::fixture_constants::{CONFIG_FILE, STATE_DIRECTORY};
const CONFIG_REVISION: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SLEEP_PROGRAM: &str = "/bin/sleep";
const CHILD_LIFETIME: &str = "30";

#[test]
fn absent_or_rejected_phase_transition_never_skips_explicit_child_settlement() {
    for already_settled in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
        let deadline = Instant::now() + yolo_remote_broker::BROKER_READY_TIMEOUT;
        let launch = ChildLaunch::new(
            root.path().join(CONFIG_FILE),
            paths.state_dir().to_owned(),
            CONFIG_REVISION.into(),
            paths.state_anchor(),
            uuid::Uuid::new_v4().to_string().parse().unwrap(),
            BrokerLaunchDeadline::capture(deadline.into()).unwrap(),
        )
        .unwrap();
        let supervision = SupervisionPaths::for_launch(&launch).unwrap();
        let parent = ProcessIdentity::capture(std::process::id()).unwrap();
        let mut record = SupervisionRecord::intent(&launch, parent, parent).unwrap();
        let reservation = reserve_termination_custody().unwrap();
        let child = yolo_process::spawn_waitable_child_before_deadline(
            Command::new(SLEEP_PROGRAM)
                .arg(CHILD_LIFETIME)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null()),
            deadline,
        )
        .unwrap();
        let child = DetachedChildCustody::capture(child).unwrap();
        let identity = child.identity().unwrap();
        if already_settled {
            record.activate(identity).unwrap();
            record.mark_settled().unwrap();
        }
        supervision.write_record(&record).unwrap();
        let before = record.phase();
        let result = after_error(
            SupervisorCustody::new(child, reservation),
            None,
            &mut record,
            &supervision,
            anyhow::anyhow!(PRIMARY_FAILURE),
        );
        assert!(result.unwrap_err().to_string().contains(PRIMARY_FAILURE));
        assert_eq!(record.phase(), before);
        assert_eq!(supervision.read_record().unwrap().phase(), before);
        assert!(
            !identity.matches(),
            "cleanup returned before captured child retirement"
        );
        let mut status = 0;
        assert_eq!(
            unsafe { libc::waitpid(identity.pid.try_into().unwrap(), &mut status, libc::WNOHANG) },
            -1
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }
}
