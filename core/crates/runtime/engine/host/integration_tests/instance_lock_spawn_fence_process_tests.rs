use crate::host::tests::*;
use super::*;

const INSTANCE_FENCE_FIXTURE_TEST: &str = "host::tests::instance_lock_spawn_fence_process_tests::instance_lock_spawn_fence_process_fixture";
const INSTANCE_FENCE_ROLE_ENV: &str = "YOLO_INSTANCE_FENCE_ROLE";
const INSTANCE_FENCE_ROOT_ENV: &str = "YOLO_INSTANCE_FENCE_ROOT";
const INSTANCE_FENCE_HOLDER_ROLE: &str = "holder";
const INSTANCE_FENCE_WORKER_ROLE: &str = "worker";
const INSTANCE_FENCE_LEAF_ROLE: &str = "leaf";
const INSTANCE_FENCE_HOME_LOCK: &str = "home.lock";
const INSTANCE_FENCE_ENDPOINT_LOCK: &str = "endpoint.lock";
const INSTANCE_FENCE_EVIDENCE: &str = "descendants.json";
const INSTANCE_FENCE_SHUTDOWN: &str = "shutdown.requested";
const INSTANCE_FENCE_HOME_LABEL: &str = "test home";
const INSTANCE_FENCE_ENDPOINT_LABEL: &str = "test endpoint";
const INSTANCE_FENCE_WAIT_ERROR: &str = "instance spawn fence fixture timed out";
const INSTANCE_FENCE_PUBLICATION_FAILURE: &str =
    "injected instance fence evidence publication failure";
const INSTANCE_FENCE_NEW_PROCESS_GROUP: i32 = 0;
const INSTANCE_FENCE_SYSCALL_FAILURE: i32 = -1;

#[derive(Clone, Copy, Deserialize, Serialize)]
struct InstanceFenceEvidence {
    worker_pid: u32,
    worker_start_time: u64,
    leaf_pid: u32,
    leaf_start_time: u64,
}

impl InstanceFenceEvidence {
    fn worker(&self) -> process::ProcessIdentity {
        process::ProcessIdentity {
            pid: self.worker_pid,
            start_time: self.worker_start_time,
        }
    }

    fn leaf(&self) -> process::ProcessIdentity {
        process::ProcessIdentity {
            pid: self.leaf_pid,
            start_time: self.leaf_start_time,
        }
    }
}

struct InstanceFenceFixture {
    holder: Option<std::process::Child>,
    holder_identity: process::ProcessIdentity,
    process_group: i32,
    evidence: Option<InstanceFenceEvidence>,
    root: tempfile::TempDir,
}

impl InstanceFenceFixture {
    fn spawn() -> Self {
        Self::spawn_with_evidence_publication(|fixture, evidence| {
            fixture.evidence = Some(evidence);
        })
    }

    fn spawn_with_evidence_publication(
        publish: impl FnOnce(&mut Self, InstanceFenceEvidence),
    ) -> Self {
        let root = tempdir().unwrap();
        let mut command = instance_fence_command(INSTANCE_FENCE_HOLDER_ROLE, root.path());
        command.process_group(INSTANCE_FENCE_NEW_PROCESS_GROUP);
        let holder = crate::process::spawn_waitable_test_child(&mut command);
        let holder_identity = process::ProcessIdentity::capture(holder.id()).unwrap();
        let process_group = verified_instance_fence_group(holder_identity);
        let mut fixture = Self {
            holder: Some(holder),
            holder_identity,
            process_group,
            evidence: None,
            root,
        };
        let evidence = wait_instance_fence_evidence(fixture.root.path());
        publish(&mut fixture, evidence);
        fixture
    }

    fn evidence(&self) -> &InstanceFenceEvidence {
        self.evidence.as_ref().unwrap()
    }

    fn stop_holder(&mut self) {
        assert!(self.holder_identity.signal(libc::SIGKILL).unwrap());
        let status = wait_instance_fence_child(self.holder.as_mut().unwrap())
            .expect(INSTANCE_FENCE_WAIT_ERROR);
        assert!(!status.success());
        self.holder = None;
        wait_instance_fence_exit(self.holder_identity);
    }

    fn stop_descendants(&self) {
        signal_instance_fence(self.evidence().leaf());
        wait_instance_fence_exit(self.evidence().leaf());
        wait_instance_fence_exit(self.evidence().worker());
    }

    fn home_lock(&self) -> PathBuf {
        self.root.path().join(INSTANCE_FENCE_HOME_LOCK)
    }

    fn endpoint_lock(&self) -> PathBuf {
        self.root.path().join(INSTANCE_FENCE_ENDPOINT_LOCK)
    }
}

impl Drop for InstanceFenceFixture {
    fn drop(&mut self) {
        if let Some(evidence) = self.evidence {
            signal_instance_fence(evidence.leaf());
        } else {
            request_instance_fence_shutdown(self.root.path());
        }
        let holder_settled = self
            .holder
            .as_mut()
            .is_none_or(|holder| wait_instance_fence_child(holder).is_some());
        if !holder_settled {
            if let Some(evidence) = self.evidence {
                signal_instance_fence(evidence.worker());
                signal_instance_fence(evidence.leaf());
                signal_instance_fence(self.holder_identity);
            } else {
                signal_instance_fence_group(self.holder_identity, self.process_group);
            }
            if let Some(holder) = self.holder.as_mut() {
                let _ = wait_instance_fence_child(holder);
            }
        }
        if let Some(evidence) = self.evidence {
            let _ = wait_instance_fence_identity(self.holder_identity);
            let _ = wait_instance_fence_identity(evidence.worker());
            let _ = wait_instance_fence_identity(evidence.leaf());
        } else {
            let _ = wait_instance_fence_group(self.holder_identity, self.process_group);
        }
    }
}

#[test]
fn host_worker_descendants_do_not_retain_instance_locks() {
    let mut fixture = InstanceFenceFixture::spawn();
    assert!(fixture.holder_identity.matches());
    assert!(fixture.evidence().worker().matches());
    assert!(fixture.evidence().leaf().matches());
    assert_instance_fence_busy(&fixture.home_lock(), INSTANCE_FENCE_HOME_LABEL);
    assert_instance_fence_busy(&fixture.endpoint_lock(), INSTANCE_FENCE_ENDPOINT_LABEL);

    fixture.stop_holder();

    assert!(fixture.evidence().worker().matches());
    assert!(fixture.evidence().leaf().matches());
    let replacements = [
        InstanceLock::acquire(&fixture.home_lock(), INSTANCE_FENCE_HOME_LABEL).unwrap(),
        InstanceLock::acquire(&fixture.endpoint_lock(), INSTANCE_FENCE_ENDPOINT_LABEL).unwrap(),
    ];
    assert!(fixture.evidence().worker().matches());
    assert!(fixture.evidence().leaf().matches());
    drop(replacements);
    fixture.stop_descendants();
}

#[test]
fn unwind_before_evidence_publication_contains_all_descendants() {
    let observed = std::cell::Cell::new(None);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        InstanceFenceFixture::spawn_with_evidence_publication(|fixture, evidence| {
            assert!(fixture.evidence.is_none());
            assert!(instance_fence_group_exists(
                fixture.holder_identity,
                fixture.process_group
            ));
            assert!(evidence.worker().matches());
            assert!(evidence.leaf().matches());
            observed.set(Some((fixture.holder_identity, fixture.process_group, evidence)));
            std::panic::panic_any(INSTANCE_FENCE_PUBLICATION_FAILURE);
        });
    }));
    let failure = outcome.unwrap_err();
    assert_eq!(
        failure.downcast_ref::<&str>().copied(),
        Some(INSTANCE_FENCE_PUBLICATION_FAILURE)
    );
    let (holder, process_group, evidence) = observed.get().unwrap();
    wait_instance_fence_exit(evidence.worker());
    wait_instance_fence_exit(evidence.leaf());
    assert!(wait_instance_fence_group(holder, process_group));
}

fn assert_instance_fence_busy(path: &Path, label: &str) {
    assert!(InstanceLock::acquire(path, label).is_err());
}

include!("instance_lock_spawn_fence_process_fixture.rs");
