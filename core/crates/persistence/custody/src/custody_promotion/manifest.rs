use crate::model::{TerminalManifest, TerminalManifestDigest};
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use anyhow::{Result, ensure};
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

const MANIFEST_DETAIL: &str = "PTY custody promotion manifest changed";

pub(super) struct ManifestFence {
    digest: TerminalManifestDigest,
    identity: FileIdentity,
}

impl ManifestFence {
    pub(super) fn capture(guard: &SessionDurableMutationGuard<'_>, paths: &Paths) -> Result<Self> {
        let (manifest, identity): (TerminalManifest, FileIdentity) = guard
            .namespace()
            .read_json_with_identity(&paths.manifest(guard.terminal_id()))?;
        ensure!(
            manifest.schema_version == crate::constants::SESSION_MANIFEST_SCHEMA_VERSION
                && manifest.terminal_id == guard.terminal_id()
                && !manifest.archived,
            MANIFEST_DETAIL
        );
        Ok(Self {
            digest: TerminalManifestDigest::capture(&manifest)?,
            identity,
        })
    }

    pub(super) fn require_exact(
        &self,
        guard: &SessionDurableMutationGuard<'_>,
        paths: &Paths,
    ) -> Result<()> {
        let (manifest, identity): (TerminalManifest, FileIdentity) = guard
            .namespace()
            .read_json_with_identity(&paths.manifest(guard.terminal_id()))?;
        ensure!(
            identity == self.identity && TerminalManifestDigest::capture(&manifest)? == self.digest,
            MANIFEST_DETAIL
        );
        Ok(())
    }
}
