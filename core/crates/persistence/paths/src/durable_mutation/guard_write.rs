use super::guard::SessionDurableMutationGuard;
use crate::paths::identity::FileIdentity;
use anyhow::Result;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurableMutationFencePhase {
    Commit,
    Installed,
}

impl SessionDurableMutationGuard<'_> {
    pub fn write_json_expected_with_commit_fence<T>(
        &self,
        path: &Path,
        expected: Option<FileIdentity>,
        value: &T,
    ) -> Result<FileIdentity>
    where
        T: serde::Serialize,
    {
        self.require_managed_path(path)?;
        self.require_no_pending_intent()?;
        let authority = self.serialized_write_authority(path)?;
        let identity = self.namespace().write_json_expected_serialized(
            path,
            expected,
            value,
            &authority,
            || {
                self.require_no_pending_intent()?;
                Ok(())
            },
            |_| {},
        )?;
        self.require_no_pending_intent()?;
        Ok(identity)
    }

    pub fn write_json_expected_with_fence<T, F>(
        &self,
        path: &Path,
        expected: Option<FileIdentity>,
        value: &T,
        fence: F,
    ) -> Result<()>
    where
        T: serde::Serialize,
        F: FnMut(DurableMutationFencePhase) -> Result<()>,
    {
        self.write_json_expected_with_fence_identity(path, expected, value, fence)?;
        Ok(())
    }

    pub fn write_json_expected_with_fence_identity<T, F>(
        &self,
        path: &Path,
        expected: Option<FileIdentity>,
        value: &T,
        mut fence: F,
    ) -> Result<FileIdentity>
    where
        T: serde::Serialize,
        F: FnMut(DurableMutationFencePhase) -> Result<()>,
    {
        self.require_managed_path(path)?;
        self.require_no_pending_intent()?;
        let authority = self.serialized_write_authority(path)?;
        let identity = self.namespace().write_json_expected_serialized(
            path,
            expected,
            value,
            &authority,
            || {
                self.require_no_pending_intent()?;
                fence(DurableMutationFencePhase::Commit)
            },
            |_| {},
        )?;
        self.require_no_pending_intent()?;
        fence(DurableMutationFencePhase::Installed)?;
        Ok(identity)
    }

    pub fn write_json_expected_with_fence_observer<T, F, G>(
        &self,
        path: &Path,
        expected: Option<FileIdentity>,
        value: &T,
        mut fence: F,
        after_commit: G,
    ) -> Result<FileIdentity>
    where
        T: serde::Serialize,
        F: FnMut(DurableMutationFencePhase) -> Result<()>,
        G: FnMut(FileIdentity),
    {
        self.require_managed_path(path)?;
        self.require_no_pending_intent()?;
        let authority = self.serialized_write_authority(path)?;
        let identity = self.namespace().write_json_expected_serialized(
            path,
            expected,
            value,
            &authority,
            || {
                self.require_no_pending_intent()?;
                fence(DurableMutationFencePhase::Commit)
            },
            after_commit,
        )?;
        self.require_no_pending_intent()?;
        fence(DurableMutationFencePhase::Installed)?;
        Ok(identity)
    }

    pub(crate) fn write_json_expected_with_fence_with_target_lock_authority<T, F>(
        &self,
        path: &Path,
        expected: Option<FileIdentity>,
        value: &T,
        authority: &crate::paths::private_file::TargetLockAuthority<'_>,
        mut fence: F,
    ) -> Result<FileIdentity>
    where
        T: serde::Serialize,
        F: FnMut(DurableMutationFencePhase) -> Result<()>,
    {
        self.require_managed_path(path)?;
        self.require_no_pending_intent()?;
        let identity = self
            .namespace()
            .write_json_expected_with_commit_fence_with_target_lock_authority(
                path,
                expected,
                value,
                authority,
                || {
                    self.require_no_pending_intent()?;
                    fence(DurableMutationFencePhase::Commit)
                },
            )?;
        self.require_no_pending_intent()?;
        fence(DurableMutationFencePhase::Installed)?;
        Ok(identity)
    }

    pub(crate) fn write_json_expected_with_fence_observer_with_target_lock_authority<T, F, G>(
        &self,
        path: &Path,
        expected: Option<FileIdentity>,
        value: &T,
        authority: &crate::paths::private_file::TargetLockAuthority<'_>,
        mut fence: F,
        after_commit: G,
    ) -> Result<FileIdentity>
    where
        T: serde::Serialize,
        F: FnMut(DurableMutationFencePhase) -> Result<()>,
        G: FnMut(FileIdentity),
    {
        self.require_managed_path(path)?;
        self.require_no_pending_intent()?;
        let identity = self
            .namespace()
            .write_json_expected_with_commit_observer_with_target_lock_authority(
                path,
                expected,
                value,
                authority,
                || {
                    self.require_no_pending_intent()?;
                    fence(DurableMutationFencePhase::Commit)
                },
                after_commit,
            )?;
        self.require_no_pending_intent()?;
        fence(DurableMutationFencePhase::Installed)?;
        Ok(identity)
    }
}
