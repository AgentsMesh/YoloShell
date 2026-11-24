use super::*;

impl ResumeTransaction {
    pub(crate) fn accepts(&self, selection: ResumeSelection) -> bool {
        if selection.intent() == ResumeIntent::Automatic {
            return true;
        }
        matches!(
            (selection, &self.target),
            (
                ResumeSelection::ExplicitRecreate,
                ResumeTarget::Shell {
                    binding: ResumeBindingDisposition::Clear { .. }
                }
            ) | (
                ResumeSelection::Preserve(ResumeIntent::Explicit),
                ResumeTarget::Shell {
                    binding: ResumeBindingDisposition::Preserve
                } | ResumeTarget::Provider { .. }
            )
        )
    }
}
