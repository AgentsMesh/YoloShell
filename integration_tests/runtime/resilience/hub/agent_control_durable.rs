use crate::Harness;
#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
use yolo_resilience_support::secondary_custody_adversarial_observation::ActiveCustodyObservation;
use yolo_runtime::{
    model::{RuntimeRecord, TerminalView},
    namespace_lease::NamespaceLease,
    paths::{Paths, identity::FileIdentity},
    pty_custody::CustodyDurableRecord,
};

const NAMESPACE_LABEL: &str = "agent control custody evidence";

pub struct Durable {
    namespace: NamespaceLease,
    runtime: RuntimeRecord,
    runtime_identity: FileIdentity,
    custody: CustodyDurableRecord,
    custody_identity: FileIdentity,
    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    guardian: ActiveCustodyObservation,
}

impl Durable {
    pub fn capture(harness: &Harness, view: &TerminalView) -> Self {
        let paths = harness.paths();
        let session = view.manifest.terminal_id;
        let namespace =
            NamespaceLease::capture(&[(NAMESPACE_LABEL, &paths.session_dir(session))]).unwrap();
        let (runtime, runtime_identity): (RuntimeRecord, _) = namespace
            .read_json_with_identity(&paths.runtime_record(session))
            .unwrap();
        let (custody, custody_identity): (CustodyDurableRecord, _) = namespace
            .read_json_with_identity(&paths.pty_custody(session))
            .unwrap();
        validate(&runtime, &custody, view);
        let holder = custody.armed_lease().unwrap().holder;
        let host = harness.host_identity();
        assert_eq!((holder.pid, holder.start_time), (host.pid, host.start_time));
        assert!(host.matches());
        #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
        let guardian = {
            let guardian = ActiveCustodyObservation::read(&paths, session).unwrap();
            assert!(guardian.matches_runtime(&runtime));
            guardian
                .confirm_retained_unchanged(&paths, session)
                .unwrap();
            guardian
        };
        Self {
            namespace,
            runtime,
            runtime_identity,
            custody,
            custody_identity,
            #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
            guardian,
        }
    }

    pub fn unchanged(&self, paths: &Paths, view: &TerminalView) {
        let session = view.manifest.terminal_id;
        let (runtime, runtime_identity): (RuntimeRecord, _) = self
            .namespace
            .read_json_with_identity(&paths.runtime_record(session))
            .unwrap();
        let (custody, custody_identity): (CustodyDurableRecord, _) = self
            .namespace
            .read_json_with_identity(&paths.pty_custody(session))
            .unwrap();
        validate(&runtime, &custody, view);
        assert_eq!(runtime_identity, self.runtime_identity);
        assert_eq!(custody_identity, self.custody_identity);
        assert_eq!(
            runtime.controller_generation,
            self.runtime.controller_generation
        );
        assert_eq!(runtime, self.runtime);
        assert_eq!(custody, self.custody);
        #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
        {
            self.guardian
                .confirm_retained_unchanged(paths, session)
                .unwrap();
            let guardian = self.guardian.guardian();
            assert!(
                crate::ProcessIdentity {
                    pid: guardian.pid,
                    start_time: guardian.start_time
                }
                .matches()
            );
            assert!(
                self.guardian
                    .guardian_custody()
                    .process_holds_pty_object(guardian.pid)
                    .unwrap()
            );
            assert!(
                crate::ProcessIdentity {
                    pid: guardian.pid,
                    start_time: guardian.start_time
                }
                .matches()
            );
        }
    }
}

fn validate(runtime: &RuntimeRecord, custody: &CustodyDurableRecord, view: &TerminalView) {
    let trusted = view.runtime.as_ref().unwrap();
    assert_eq!(runtime.terminal_id, view.manifest.terminal_id);
    assert_eq!(runtime.runtime_epoch, trusted.runtime_epoch);
    assert_eq!(runtime.worker_id, trusted.worker_id);
    assert_eq!(
        (runtime.worker_pid, runtime.worker_start_time),
        (trusted.worker_pid, trusted.worker_start_time)
    );
    assert_eq!(
        (runtime.child_pid, runtime.child_start_time),
        (trusted.child_pid, trusted.child_start_time)
    );
    assert_eq!(runtime.bootstrap_delivery, trusted.bootstrap_delivery);
    assert!(custody.is_valid());
    assert!(custody.runtime.matches_record(runtime));
    assert_eq!(
        custody.armed_lease().unwrap().controller_generation,
        runtime.controller_generation
    );
}
