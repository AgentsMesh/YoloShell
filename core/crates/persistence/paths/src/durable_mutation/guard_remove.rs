use super::guard::SessionDurableMutationGuard;
use crate::paths::identity::FileIdentity;
use anyhow::{Result, ensure};
use std::path::Path;

const GUARDED_REMOVE_CHANGED: &str = "guarded durable removal target changed";

impl SessionDurableMutationGuard<'_> {
    pub fn remove_exact_with_replay_boundary<F>(
        &self,
        path: &Path,
        expected: FileIdentity,
        max_bytes: u64,
        mut verify_replay_boundary: F,
    ) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        self.require_managed_path(path)?;
        self.require_no_pending_intent()?;
        let _parent_authority = self.serialized_write_authority(path)?;
        let entry = self.namespace().file(path)?;
        match entry.inspect(max_bytes)? {
            Some(current) => {
                ensure!(current.unchanged(expected), GUARDED_REMOVE_CHANGED);
                entry.remove_if_unchanged_with_fence(expected, || {
                    self.require_no_pending_intent()?;
                    verify_replay_boundary()?;
                    ensure!(
                        entry
                            .inspect(max_bytes)?
                            .is_some_and(|current| current.unchanged(expected)),
                        GUARDED_REMOVE_CHANGED
                    );
                    Ok(())
                })?;
            }
            None => {
                verify_replay_boundary()?;
                self.require_no_pending_intent()?;
                ensure!(entry.inspect(max_bytes)?.is_none(), GUARDED_REMOVE_CHANGED);
                verify_replay_boundary()?;
            }
        }
        self.require_no_pending_intent()?;
        ensure!(entry.inspect(max_bytes)?.is_none(), GUARDED_REMOVE_CHANGED);
        Ok(())
    }
}
