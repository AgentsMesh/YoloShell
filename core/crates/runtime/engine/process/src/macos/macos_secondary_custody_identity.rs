use super::*;

const IDENTITY_DIGEST_DOMAIN: &str = "yolo/macos-secondary-custody-identity/v1";

impl MacosSecondaryCustodyIdentity {
    pub(super) fn capture(
        runtime: &CustodyRuntimeIdentity,
        pty_master: &MacosPassivePtyMaster,
        process_capability: MacosProcessCapabilityIdentity,
    ) -> Self {
        Self::from_observed(
            runtime,
            pty_master.slave_path().to_owned(),
            pty_master.object_identity(),
            process_capability,
        )
    }

    pub(in crate::process) fn from_observed(
        runtime: &CustodyRuntimeIdentity,
        pty_slave_path: PathBuf,
        pty_object: MacosPtyObjectIdentity,
        process_capability: MacosProcessCapabilityIdentity,
    ) -> Self {
        Self {
            runtime: runtime.clone(),
            pty_slave_path,
            pty_object,
            process_capability,
        }
    }

    pub(in crate::process) fn digest(&self) -> Result<Sha256Digest> {
        Sha256Digest::capture(&(IDENTITY_DIGEST_DOMAIN, self))
    }
}
