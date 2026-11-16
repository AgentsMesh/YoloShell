use crate::process::{
    MacosProcessCapabilityChildWriter, MacosProcessCapabilityIssuer, MacosSecondaryCustody,
    MacosSecondaryCustodyExport, PtySessionProof,
};
use portable_pty::{PtyPair, PtySize, SlavePty, native_pty_system};
use yolo_pty_custody::{CustodyProfile, CustodyRuntimeIdentity};

#[doc(hidden)]
pub struct ActiveMacosSecondaryCustodyFixture {
    pair: PtyPair,
    proof: PtySessionProof,
    writer: MacosProcessCapabilityChildWriter,
}

#[doc(hidden)]
pub struct MacosSecondaryCustodyFixtureAliases {
    _pair: PtyPair,
    writer: MacosProcessCapabilityChildWriter,
}

impl MacosSecondaryCustodyFixtureAliases {
    pub fn writer_handle(&self) -> u64 {
        self.writer.identity().writer_handle()
    }
}

impl ActiveMacosSecondaryCustodyFixture {
    pub fn new(size: PtySize, capability: &str) -> Self {
        let pair = native_pty_system().openpty(size).unwrap();
        let mut proof =
            PtySessionProof::capture_bound(pair.master.as_ref(), capability.to_owned()).unwrap();
        let (reader, writer) = MacosProcessCapabilityIssuer::create()
            .unwrap()
            .into_staged()
            .unwrap();
        proof.install_process_capability(reader).unwrap();
        Self {
            pair,
            proof,
            writer,
        }
    }

    pub fn configure_exec_inheritance(&self, command: &mut std::process::Command) {
        self.writer.configure_exec_inheritance(command);
    }

    pub fn pty_path(&self) -> std::path::PathBuf {
        self.pair.master.tty_name().unwrap()
    }

    pub fn capture(self, runtime: &CustodyRuntimeIdentity) -> MacosSecondaryCustody {
        self.capture_retaining_slave(runtime).0
    }

    pub fn export(
        mut self,
        runtime: &CustodyRuntimeIdentity,
    ) -> (
        MacosSecondaryCustodyFixtureAliases,
        MacosSecondaryCustodyExport,
    ) {
        let alias = self
            .proof
            .bind_secondary_custody_alias_for_test(runtime)
            .unwrap();
        let export = MacosSecondaryCustodyExport::capture(&self.proof, runtime).unwrap();
        drop((alias, self.proof));
        (
            MacosSecondaryCustodyFixtureAliases {
                _pair: self.pair,
                writer: self.writer,
            },
            export,
        )
    }

    pub fn capture_retaining_slave(
        mut self,
        runtime: &CustodyRuntimeIdentity,
    ) -> (MacosSecondaryCustody, Box<dyn SlavePty + Send>) {
        let alias = self
            .proof
            .bind_secondary_custody_alias_for_test(runtime)
            .unwrap();
        let export = MacosSecondaryCustodyExport::capture(&self.proof, runtime).unwrap();
        let digest = export.recovery_identity_sha256();
        let descriptors = export.descriptor_set();
        let custody = MacosSecondaryCustody::from_transferred(
            export.into_parts(),
            runtime,
            digest,
            descriptors,
            CustodyProfile::ActiveRunning,
        )
        .unwrap();
        drop((alias, self.writer, self.proof, self.pair.master));
        (custody, self.pair.slave)
    }
}
