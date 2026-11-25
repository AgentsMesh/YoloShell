use crate::pty_master_closure_contract as contract;
use yolo_architecture_test_support::Fixture;

const POLICY_MESSAGE_PREFIX: &str = "PTY master-closure capabil";

pub const SETTLEMENT_SOURCE: &str = r#"
pub(crate) struct PtyLocalMasterAliasesClosed {}
pub(crate) struct PtyHostReleaseVerifier {}
pub(crate) struct PtyHostReleasedAck {}
pub(crate) struct PtyMasterAliasClosureSettlement {}
pub(crate) struct PtyMasterAliasesClosed {}
impl PtyMasterAliasReleaseAuthority {
    pub fn take_local_aliases_closed(&self) -> (PtyLocalMasterAliasesClosed, PtyHostReleaseVerifier) {
        (PtyLocalMasterAliasesClosed {}, PtyHostReleaseVerifier {})
    }
}
impl PtyHostReleaseVerifier {
    pub fn acknowledge(self, _exact: ExactCustodyReleasedAck) -> PtyHostReleasedAck {
        self.acknowledgement()
    }
    pub fn acknowledge_holder_gone(self, _exact: ExactCustodyHolderGone) -> PtyHostReleasedAck {
        self.acknowledgement()
    }
    fn acknowledgement(self) -> PtyHostReleasedAck {
        PtyHostReleasedAck {}
    }
}
impl PtyLocalMasterAliasesClosed {
    pub fn finalize_host_release(self, _ack: PtyHostReleasedAck) -> PtyMasterAliasClosureSettlement {
        PtyMasterAliasClosureSettlement {}
    }
    pub fn finalize_without_secondary_custody(self) -> PtyMasterAliasClosureSettlement {
        PtyMasterAliasClosureSettlement {}
    }
}
impl PtyMasterAliasClosureSettlement {
    pub fn mint_final_proof(self) -> PtyMasterAliasesClosed {
        PtyMasterAliasesClosed {}
    }
}
"#;

pub const OUTPUT_SOURCE: &str = r#"
impl PtyOutputRecovery {
    fn release_master_aliases(&mut self) {
        runtime.release(|| { authority.take_local_aliases_closed() });
    }
}
"#;

pub const STARTUP_CUSTODY_SOURCE: &str = r#"
use crate::process::PtyHostReleaseVerifier;
impl ExclusivePtyOutput {
    fn prove_host_release(&mut self, exact: ExactCustodyReleasedAck) {
        let acknowledgement = verifier.clone().acknowledge(exact);
        let proof = local.finalize_host_release(acknowledgement).unwrap().mint_final_proof();
        consume(proof);
    }
    fn close_local_aliases(&mut self) {
        authority.take_local_aliases_closed();
    }
}
"#;

pub const STARTUP_CUSTODY_RELEASE_SOURCE: &str = r#"
impl ExclusivePtyOutput {
    fn begin_pending_or_restore(&mut self) {
        authority.take_local_aliases_closed();
    }
    fn poll_holder_gone(&mut self, gone: ExactCustodyHolderGone) {
        let acknowledgement = verifier.clone().acknowledge_holder_gone(gone);
        let proof = local.finalize_host_release(acknowledgement).unwrap().mint_final_proof();
        consume(proof);
    }
    fn release_local_only_or_restore(&mut self) {
        let proof = authority.take_local_aliases_closed().0
            .finalize_without_secondary_custody().mint_final_proof();
        consume(proof);
    }
}
"#;

pub const RUNTIME_SOURCE: &str = r#"
impl PtyRuntimeAliasCustody {
    fn poll_completion(&self) {
        let proof = local.finalize_host_release(acknowledgement)
            .unwrap().mint_final_proof();
        consume(proof);
    }
}
"#;

pub const ACK_SOURCE: &str = r#"
use crate::process::PtyHostReleaseVerifier;
struct WorkerCustodyReleaseCommand { verifier: PtyHostReleaseVerifier }
impl WorkerCustodyReleaseCommand {
    fn acknowledge(self, exact: ExactCustodyReleasedAck) {
        let acknowledgement = self.verifier.acknowledge(exact);
        consume(acknowledgement);
    }
}
"#;

pub fn contract_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(contract::SETTLEMENT_PATH, SETTLEMENT_SOURCE);
    fixture.write(contract::OUTPUT_RELEASE_PATH, OUTPUT_SOURCE);
    fixture.write(contract::RUNTIME_RELEASE_PATH, RUNTIME_SOURCE);
    fixture.write(contract::CUSTODY_RELEASE_PATH, ACK_SOURCE);
    fixture.write(contract::STARTUP_CUSTODY_PATH, STARTUP_CUSTODY_SOURCE);
    fixture.write(
        contract::STARTUP_CUSTODY_RELEASE_PATH,
        STARTUP_CUSTODY_RELEASE_SOURCE,
    );
    fixture
}

pub fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .filter(|message| message.contains(POLICY_MESSAGE_PREFIX))
        .collect()
}
