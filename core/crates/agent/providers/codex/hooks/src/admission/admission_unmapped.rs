use super::{CodexExecutableAttestation, CodexExecutableKind};

const UNSUPPORTED: &str = "Codex hook executable mapping is unsupported on this platform";

impl CodexExecutableAttestation {
    pub fn verify_mapped_process(
        &self,
        _pid: u32,
        _kind: CodexExecutableKind,
    ) -> anyhow::Result<()> {
        anyhow::bail!(UNSUPPORTED)
    }
}
