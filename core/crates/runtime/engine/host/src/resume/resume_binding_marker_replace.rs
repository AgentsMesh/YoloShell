use super::*;

impl Host {
    pub(crate) fn replace_resume_binding_marker(
        &self,
        expected: &ResumeBindingClearMarker,
        replacement: &ResumeBindingClearMarker,
    ) -> Result<()> {
        let identity = self.resume_binding_marker_identity(expected, replacement)?;
        self.paths.replace_resume_binding_marker(
            self.namespace.as_ref(),
            expected.terminal_id,
            identity,
            replacement,
        )
    }

    #[cfg(test)]
    pub(crate) fn replace_resume_binding_marker_with_boundary<F>(
        &self,
        expected: &ResumeBindingClearMarker,
        replacement: &ResumeBindingClearMarker,
        boundary: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        let identity = self.resume_binding_marker_identity(expected, replacement)?;
        boundary()?;
        self.paths.replace_resume_binding_marker(
            self.namespace.as_ref(),
            expected.terminal_id,
            identity,
            replacement,
        )
    }

    pub(crate) fn resume_binding_marker_identity(
        &self,
        expected: &ResumeBindingClearMarker,
        replacement: &ResumeBindingClearMarker,
    ) -> Result<crate::paths::identity::FileIdentity> {
        anyhow::ensure!(
            !expected.terminal_id.is_nil() && expected.terminal_id == replacement.terminal_id,
            RESUME_BINDING_MARKER_INVALID
        );
        let path = self.paths.resume_binding_clear(expected.terminal_id);
        let entry = self.namespace.file(&path)?;
        let (current, identity): (ResumeBindingClearMarker, _) =
            crate::paths::read_entry_guarded_with_identity(entry, || self.namespace.verify())?;
        self.namespace.verify()?;
        anyhow::ensure!(current == *expected, RESUME_BINDING_MARKER_INVALID);
        Ok(identity)
    }
}
