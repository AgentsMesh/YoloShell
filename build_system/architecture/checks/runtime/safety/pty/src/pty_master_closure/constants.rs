#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Occurrence {
    Definition,
    Call,
    Constructor,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Route {
    Definition,
    LocalClose,
    HostAcknowledgement,
    StartupHostAcknowledgement,
    HolderGoneAcknowledgement,
    HostSettlement,
    NoSecondaryCustody,
}

pub struct Site {
    pub symbol: &'static str,
    pub path: &'static str,
    pub owner: &'static str,
    pub function: &'static str,
    pub occurrence: Occurrence,
    pub route: Route,
}

pub const SETTLEMENT_PATH: &str =
    "core/crates/runtime/engine/process/src/pty/pty_master_release_settlement.rs";
pub const OUTPUT_RELEASE_PATH: &str = "core/crates/runtime/engine/worker/src/pty/output/pty_output_release.rs";
pub const RUNTIME_RELEASE_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/alias/pty_runtime_alias_release.rs";
pub const CUSTODY_RELEASE_PATH: &str =
    "core/crates/runtime/engine/worker/src/secondary_custody/secondary_custody_release.rs";
pub const STARTUP_CUSTODY_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/output/pty_output_startup_custody.rs";
pub const STARTUP_CUSTODY_RELEASE_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/output/pty_output_startup_custody_release.rs";
pub const AUTHORITY: &str = "PtyMasterAliasReleaseAuthority";
pub const LOCAL_PROOF: &str = "PtyLocalMasterAliasesClosed";
pub const HOST_VERIFIER: &str = "PtyHostReleaseVerifier";
pub const HOST_ACK: &str = "PtyHostReleasedAck";
pub const SETTLEMENT: &str = "PtyMasterAliasClosureSettlement";
pub const FINAL_PROOF: &str = "PtyMasterAliasesClosed";
pub const RECOVERY: &str = "PtyOutputRecovery";
pub const EXCLUSIVE_OUTPUT: &str = "ExclusivePtyOutput";
pub const RUNTIME_CUSTODY: &str = "PtyRuntimeAliasCustody";
pub const RELEASE_COMMAND: &str = "WorkerCustodyReleaseCommand";
pub const RELEASE_ALIASES: &str = "release_master_aliases";
pub const POLL_COMPLETION: &str = "poll_completion";
pub const PROVE_HOST_RELEASE: &str = "prove_host_release";
pub const CLOSE_LOCAL_ALIASES: &str = "close_local_aliases";
pub const BEGIN_PENDING: &str = "begin_pending_or_restore";
pub const POLL_HOLDER_GONE: &str = "poll_holder_gone";
pub const RELEASE_LOCAL_ONLY: &str = "release_local_only_or_restore";
pub const TAKE_LOCAL: &str = "take_local_aliases_closed";
pub const ACKNOWLEDGE: &str = "acknowledge";
pub const ACKNOWLEDGE_HOLDER_GONE: &str = "acknowledge_holder_gone";
pub const ACKNOWLEDGEMENT: &str = "acknowledgement";
pub const FINALIZE_HOST: &str = "finalize_host_release";
pub const FINALIZE_LOCAL: &str = "finalize_without_secondary_custody";
pub const MINT_FINAL: &str = "mint_final_proof";
pub const SELF_RECEIVER: &str = "self";
pub const SELF_TYPE: &str = "Self";
pub const VERIFIER_FIELD: &str = "verifier";
pub const EXACT_ACK_ARGUMENT: &str = "exact";
pub const HOLDER_GONE_ARGUMENT: &str = "gone";
pub const POLICY_MESSAGE: &str =
    "PTY master-closure capabilities are restricted to the typed settlement routes";
pub const CONTRACT_MESSAGE: &str =
    "PTY master-closure capability definition or consumption contract changed";
pub const EXPECTED_SITE_OCCURRENCES: usize = 1;

const fn site(
    symbol: &'static str,
    path: &'static str,
    owner: &'static str,
    function: &'static str,
    occurrence: Occurrence,
    route: Route,
) -> Site {
    Site {
        symbol,
        path,
        owner,
        function,
        occurrence,
        route,
    }
}

#[rustfmt::skip]
pub const METHOD_SITES: &[Site] = &[
    site(TAKE_LOCAL, SETTLEMENT_PATH, AUTHORITY, TAKE_LOCAL, Occurrence::Definition, Route::Definition),
    site(ACKNOWLEDGE, SETTLEMENT_PATH, HOST_VERIFIER, ACKNOWLEDGE, Occurrence::Definition, Route::Definition),
    site(ACKNOWLEDGE_HOLDER_GONE, SETTLEMENT_PATH, HOST_VERIFIER, ACKNOWLEDGE_HOLDER_GONE, Occurrence::Definition, Route::Definition),
    site(FINALIZE_HOST, SETTLEMENT_PATH, LOCAL_PROOF, FINALIZE_HOST, Occurrence::Definition, Route::Definition),
    site(FINALIZE_LOCAL, SETTLEMENT_PATH, LOCAL_PROOF, FINALIZE_LOCAL, Occurrence::Definition, Route::Definition),
    site(MINT_FINAL, SETTLEMENT_PATH, SETTLEMENT, MINT_FINAL, Occurrence::Definition, Route::Definition),
    site(TAKE_LOCAL, OUTPUT_RELEASE_PATH, RECOVERY, RELEASE_ALIASES, Occurrence::Call, Route::LocalClose),
    site(ACKNOWLEDGE, CUSTODY_RELEASE_PATH, RELEASE_COMMAND, ACKNOWLEDGE, Occurrence::Call, Route::HostAcknowledgement),
    site(FINALIZE_HOST, RUNTIME_RELEASE_PATH, RUNTIME_CUSTODY, POLL_COMPLETION, Occurrence::Call, Route::HostSettlement),
    site(MINT_FINAL, RUNTIME_RELEASE_PATH, RUNTIME_CUSTODY, POLL_COMPLETION, Occurrence::Call, Route::HostSettlement),
    site(ACKNOWLEDGE, STARTUP_CUSTODY_PATH, EXCLUSIVE_OUTPUT, PROVE_HOST_RELEASE, Occurrence::Call, Route::StartupHostAcknowledgement),
    site(FINALIZE_HOST, STARTUP_CUSTODY_PATH, EXCLUSIVE_OUTPUT, PROVE_HOST_RELEASE, Occurrence::Call, Route::HostSettlement),
    site(MINT_FINAL, STARTUP_CUSTODY_PATH, EXCLUSIVE_OUTPUT, PROVE_HOST_RELEASE, Occurrence::Call, Route::HostSettlement),
    site(TAKE_LOCAL, STARTUP_CUSTODY_PATH, EXCLUSIVE_OUTPUT, CLOSE_LOCAL_ALIASES, Occurrence::Call, Route::LocalClose),
    site(TAKE_LOCAL, STARTUP_CUSTODY_RELEASE_PATH, EXCLUSIVE_OUTPUT, BEGIN_PENDING, Occurrence::Call, Route::LocalClose),
    site(ACKNOWLEDGE_HOLDER_GONE, STARTUP_CUSTODY_RELEASE_PATH, EXCLUSIVE_OUTPUT, POLL_HOLDER_GONE, Occurrence::Call, Route::HolderGoneAcknowledgement),
    site(FINALIZE_HOST, STARTUP_CUSTODY_RELEASE_PATH, EXCLUSIVE_OUTPUT, POLL_HOLDER_GONE, Occurrence::Call, Route::HostSettlement),
    site(MINT_FINAL, STARTUP_CUSTODY_RELEASE_PATH, EXCLUSIVE_OUTPUT, POLL_HOLDER_GONE, Occurrence::Call, Route::HostSettlement),
    site(TAKE_LOCAL, STARTUP_CUSTODY_RELEASE_PATH, EXCLUSIVE_OUTPUT, RELEASE_LOCAL_ONLY, Occurrence::Call, Route::NoSecondaryCustody),
    site(FINALIZE_LOCAL, STARTUP_CUSTODY_RELEASE_PATH, EXCLUSIVE_OUTPUT, RELEASE_LOCAL_ONLY, Occurrence::Call, Route::NoSecondaryCustody),
    site(MINT_FINAL, STARTUP_CUSTODY_RELEASE_PATH, EXCLUSIVE_OUTPUT, RELEASE_LOCAL_ONLY, Occurrence::Call, Route::NoSecondaryCustody),
];

#[rustfmt::skip]
pub const CONSTRUCTOR_SITES: &[Site] = &[
    site(LOCAL_PROOF, SETTLEMENT_PATH, AUTHORITY, TAKE_LOCAL, Occurrence::Constructor, Route::Definition),
    site(HOST_VERIFIER, SETTLEMENT_PATH, AUTHORITY, TAKE_LOCAL, Occurrence::Constructor, Route::Definition),
    site(HOST_ACK, SETTLEMENT_PATH, HOST_VERIFIER, ACKNOWLEDGEMENT, Occurrence::Constructor, Route::Definition),
    site(SETTLEMENT, SETTLEMENT_PATH, LOCAL_PROOF, FINALIZE_HOST, Occurrence::Constructor, Route::Definition),
    site(SETTLEMENT, SETTLEMENT_PATH, LOCAL_PROOF, FINALIZE_LOCAL, Occurrence::Constructor, Route::Definition),
    site(FINAL_PROOF, SETTLEMENT_PATH, SETTLEMENT, MINT_FINAL, Occurrence::Constructor, Route::Definition),
];
