use super::ProcessIdentity;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use uuid::Uuid;

#[cfg(target_os = "macos")]
#[path = "pty_master_release_fallback_macos.rs"]
mod fallback;
#[path = "pty_master_release_output_binding.rs"]
mod output_binding;
#[path = "pty_master_release_settlement.rs"]
mod settlement;
pub use settlement::{
    PtyHostReleaseVerifier, PtyHostReleasedAck, PtyLocalMasterAliasesClosed,
    PtyMasterAliasClosureSettlement,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PtyRuntimeIdentity {
    terminal_id: Uuid,
    runtime_epoch: Uuid,
}

#[derive(Clone, Debug)]
pub struct ProcessClosureIdentity(Arc<ProcessClosureAllocation>);

#[derive(Debug)]
struct ProcessClosureAllocation {
    _private: (),
}

pub struct PtyMasterAliasReleaseAuthority {
    identity: Arc<PtyClosureIdentity>,
    local_proof_available: AtomicBool,
}

pub(super) struct PtyMasterAliasVerifier {
    identity: Arc<PtyClosureIdentity>,
}

pub struct PtyMasterAliasesClosed {
    identity: PtyMasterAliasesClosedIdentity,
    #[cfg(any(test, feature = "runtime-component-tests"))]
    drop_panic: Option<&'static str>,
}

struct PtyClosureIdentity {
    process_closure: ProcessClosureIdentity,
    runtime: PtyRuntimeIdentity,
    root: ProcessIdentity,
    capability: String,
    object: PtyObjectIdentity,
    #[cfg(target_os = "macos")]
    fallback: std::sync::Mutex<fallback::ConfirmedBootstrapFallbackState>,
}

enum PtyMasterAliasesClosedIdentity {
    Closure(Arc<PtyClosureIdentity>),
    #[cfg(any(test, feature = "runtime-component-tests"))]
    Scripted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum PtyObjectIdentity {
    #[cfg(target_os = "linux")]
    LinuxDevice(u64),
    #[cfg(target_os = "macos")]
    Macos(super::pty_session_proof::MacosPtyObjectIdentity),
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    PortablePath(std::path::PathBuf),
}

impl PtyRuntimeIdentity {
    pub(crate) fn new(terminal_id: Uuid, runtime_epoch: Uuid) -> Self {
        Self {
            terminal_id,
            runtime_epoch,
        }
    }
}

impl ProcessClosureIdentity {
    pub(crate) fn fresh() -> Self {
        Self(Arc::new(ProcessClosureAllocation { _private: () }))
    }

    pub(super) fn same_allocation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl PartialEq for ProcessClosureIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.same_allocation(other)
    }
}

impl Eq for ProcessClosureIdentity {}

impl PtyMasterAliasReleaseAuthority {
    pub(super) fn new_pair(
        runtime: PtyRuntimeIdentity,
        root: ProcessIdentity,
        capability: String,
        object: PtyObjectIdentity,
    ) -> (Self, PtyMasterAliasVerifier) {
        let identity = Arc::new(PtyClosureIdentity {
            process_closure: ProcessClosureIdentity::fresh(),
            runtime,
            root,
            capability,
            object,
            #[cfg(target_os = "macos")]
            fallback: std::sync::Mutex::new(fallback::ConfirmedBootstrapFallbackState::default()),
        });
        (
            Self {
                identity: Arc::clone(&identity),
                local_proof_available: AtomicBool::new(true),
            },
            PtyMasterAliasVerifier { identity },
        )
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn prove_closed_for_test(&self) -> PtyMasterAliasesClosed {
        self.take_local_aliases_closed()
            .expect("test PTY master-alias closure proof is available")
            .0
            .finalize_without_secondary_custody()
            .mint_final_proof()
    }
}

impl PtyMasterAliasVerifier {
    pub(super) fn process_closure_identity(&self) -> ProcessClosureIdentity {
        self.identity.process_closure.clone()
    }
}

impl PtyMasterAliasesClosed {
    pub(super) fn validates(&self, expected: &PtyMasterAliasVerifier) -> bool {
        match &self.identity {
            PtyMasterAliasesClosedIdentity::Closure(identity) => {
                Arc::ptr_eq(identity, &expected.identity)
            }
            #[cfg(any(test, feature = "runtime-component-tests"))]
            PtyMasterAliasesClosedIdentity::Scripted => false,
        }
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn scripted() -> Self {
        Self {
            identity: PtyMasterAliasesClosedIdentity::Scripted,
            drop_panic: None,
        }
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn scripted_with_drop_panic(detail: &'static str) -> Self {
        Self {
            identity: PtyMasterAliasesClosedIdentity::Scripted,
            drop_panic: Some(detail),
        }
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) fn is_scripted(&self) -> bool {
        matches!(self.identity, PtyMasterAliasesClosedIdentity::Scripted)
    }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/pty/master/pty_master_release_fallback_macos_tests.rs"]
mod fallback_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/pty/master/pty_master_release_settlement_tests.rs"]
mod settlement_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/pty/master/pty_master_release_tests.rs"]
mod tests;

#[cfg(any(test, feature = "runtime-component-tests"))]
impl Drop for PtyMasterAliasesClosed {
    fn drop(&mut self) {
        if let Some(detail) = self.drop_panic.take() {
            panic!("{detail}");
        }
    }
}
