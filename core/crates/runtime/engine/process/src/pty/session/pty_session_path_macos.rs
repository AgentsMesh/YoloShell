use super::PtySessionProof;
use super::macos_contract::{CapabilityProofError, CapabilityProofResult};
use super::macos_object_identity::MacosPtyObjectIdentity;
use anyhow::{anyhow, ensure};
use std::os::unix::fs::FileTypeExt;

const TTY_METADATA_CONTEXT: &str = "revalidate PTY slave path metadata";
const TTY_TYPE_DETAIL: &str = "PTY slave path is no longer a character device";
const TTY_IDENTITY_DETAIL: &str = "PTY slave path object identity changed";

#[derive(Clone, Copy)]
struct TtyPathIdentity {
    is_character_device: bool,
    object: MacosPtyObjectIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TtyPathState {
    Available(MacosPtyObjectIdentity),
    Released,
}

impl PtySessionProof {
    pub(super) fn tty_path_state(&self) -> CapabilityProofResult<TtyPathState> {
        let metadata = match std::fs::metadata(&self.tty_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(TtyPathState::Released);
            }
            Err(error) => {
                return Err(CapabilityProofError::invariant(
                    anyhow::Error::new(error).context(TTY_METADATA_CONTEXT),
                ));
            }
        };
        let observed = tty_identity(metadata);
        validate_tty_identity(self.pty_object_identity()?, observed)
            .map_err(CapabilityProofError::invariant)?;
        Ok(TtyPathState::Available(observed.object))
    }
}

fn tty_identity(metadata: std::fs::Metadata) -> TtyPathIdentity {
    TtyPathIdentity {
        is_character_device: metadata.file_type().is_char_device(),
        object: MacosPtyObjectIdentity::from_metadata(&metadata),
    }
}

fn validate_tty_identity(
    expected: MacosPtyObjectIdentity,
    observed: TtyPathIdentity,
) -> anyhow::Result<()> {
    ensure!(observed.is_character_device, TTY_TYPE_DETAIL);
    if observed.object != expected {
        return Err(anyhow!(TTY_IDENTITY_DETAIL));
    }
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/path/pty_session_path_macos_tests.rs"]
mod tests;
