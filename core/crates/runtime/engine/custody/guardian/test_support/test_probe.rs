use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::process::PtyMasterFdCustodyProbe;
use crate::pty_custody::{CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderIdentity};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[cfg(any(feature = "yolo-unit-tests", feature = "adversarial-test-hooks"))]
#[path = "test_probe_observation.rs"]
mod observation;
#[cfg(all(test, feature = "yolo-unit-tests"))]
pub(crate) use observation::read_exact_session as read_exact;
#[cfg(feature = "adversarial-test-hooks")]
pub use observation::{ExactProbeObservation, read_exact_observation};

const PROBE_SCHEMA_VERSION: u16 = 2;
const PROBE_DIRECTORY_NAME: &str = "guardian-pty-custody-probes";
const PROBE_FILE_SUFFIX: &str = ".json";
const ARMED_RECORD_REQUIRED: &str = "Guardian custody probe requires exact armed custody";
const CURRENT_GUARDIAN_REQUIRED: &str =
    "Guardian custody probe publisher does not match the durable guardian";
const EXACT_DESCRIPTOR_REQUIRED: &str = "Guardian custody probe has no exact PTY descriptor";
const DURABLE_RECORD_CHANGED: &str = "Guardian custody probe durable record changed";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProbeBinding {
    schema_version: u16,
    #[serde(with = "crate::serde_non_nil_uuid")]
    guardian_id: Uuid,
    guardian_process: CustodyHolderIdentity,
    controller_generation: u64,
    durable_record_identity: FileIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct GuardianCustodyProbe {
    binding: ProbeBinding,
    custody: PtyMasterFdCustodyProbe,
}

impl ProbeBinding {
    fn capture(record: &CustodyDurableRecord, identity: FileIdentity) -> Result<Self> {
        let lease = record.armed_lease().context(ARMED_RECORD_REQUIRED)?;
        let guardian = lease.guardian.context(ARMED_RECORD_REQUIRED)?;
        ensure!(
            record.is_valid() && guardian.matches_durable_lease(&record.runtime, &lease),
            ARMED_RECORD_REQUIRED
        );
        Ok(Self {
            schema_version: PROBE_SCHEMA_VERSION,
            guardian_id: guardian.guardian_id(),
            guardian_process: guardian.process(),
            controller_generation: lease.controller_generation,
            durable_record_identity: identity,
        })
    }

    fn matches(
        self,
        record: &CustodyDurableRecord,
        identity: FileIdentity,
    ) -> Option<CustodyGuardianBinding> {
        let lease = record.armed_lease()?;
        let guardian = lease.guardian?;
        (self.schema_version == PROBE_SCHEMA_VERSION
            && record.is_valid()
            && guardian.matches_durable_lease(&record.runtime, &lease)
            && self.guardian_id == guardian.guardian_id()
            && self.guardian_process == guardian.process()
            && self.controller_generation == lease.controller_generation
            && self.durable_record_identity == identity)
            .then_some(guardian)
    }

    fn matches_guardian(self, guardian_id: Uuid, guardian_process: CustodyHolderIdentity) -> bool {
        self.guardian_id == guardian_id && self.guardian_process == guardian_process
    }
}

pub(super) fn publish_armed(
    namespace: &NamespaceLease,
    paths: &Paths,
    record: &CustodyDurableRecord,
    identity: FileIdentity,
    publisher: CustodyGuardianBinding,
    custody: PtyMasterFdCustodyProbe,
) -> Result<()> {
    let binding = ProbeBinding::capture(record, identity)?;
    ensure!(
        binding.matches_guardian(publisher.guardian_id(), publisher.process()),
        CURRENT_GUARDIAN_REQUIRED
    );
    ensure!(
        binding.guardian_process.matches()
            && custody.process_holds_pty_object(binding.guardian_process.pid)?,
        EXACT_DESCRIPTOR_REQUIRED
    );
    let probe = GuardianCustodyProbe { binding, custody };
    namespace.ensure_directory(&probe_directory(paths, record.runtime.terminal_id))?;
    let target = path(paths, record.runtime.terminal_id, binding.guardian_id);
    if namespace.read_json::<GuardianCustodyProbe>(&target).ok() != Some(probe.clone()) {
        namespace.write_json(&target, &probe)?;
    }
    confirm_durable_unchanged(namespace, paths, record, identity)
}

pub fn confirm_durable_unchanged(
    namespace: &NamespaceLease,
    paths: &Paths,
    expected: &CustodyDurableRecord,
    expected_identity: FileIdentity,
) -> Result<()> {
    let current: (CustodyDurableRecord, FileIdentity) =
        namespace.read_json_with_identity(&paths.pty_custody(expected.runtime.terminal_id))?;
    ensure!(
        current == (expected.clone(), expected_identity),
        DURABLE_RECORD_CHANGED
    );
    Ok(())
}

fn path(paths: &Paths, terminal_id: Uuid, guardian_id: Uuid) -> PathBuf {
    probe_directory(paths, terminal_id).join(format!("{guardian_id}{PROBE_FILE_SUFFIX}"))
}

fn probe_directory(paths: &Paths, terminal_id: Uuid) -> PathBuf {
    paths.session_dir(terminal_id).join(PROBE_DIRECTORY_NAME)
}

#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/probe/test_probe_tests.rs"]
mod tests;
