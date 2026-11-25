use super::guardians::BoundCustody;
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SessionAuthorityPhase {
    Active,
    Retained,
}

impl SessionAuthorityPhase {
    fn confirm_guardian(self, guardian: &BoundCustody, paths: &Paths) -> Result<()> {
        match self {
            Self::Active => guardian.confirm_active(paths),
            Self::Retained => guardian.confirm_retained(paths),
        }
    }
}

pub(crate) fn confirm_session_authority(
    guardians: &[BoundCustody],
    paths: &Paths,
    record: &BoundRuntimeRecord,
    phase: SessionAuthorityPhase,
) -> Result<()> {
    super::validation::validate_targets(paths, std::slice::from_ref(record))?;
    let terminal_id = record.record().terminal_id;
    match guardians
        .iter()
        .find(|guardian| guardian.terminal_id() == terminal_id)
    {
        Some(guardian) => phase.confirm_guardian(guardian, paths),
        None => super::guardians::confirm_absent(paths, terminal_id),
    }
}

impl PowerLossFreezeBarrier {
    pub(super) fn confirm_guardians(&self, paths: &Paths) -> Result<()> {
        self.guardians
            .iter()
            .try_for_each(|guardian| guardian.confirm_active(paths))
    }

    pub(super) fn confirm_session_authority(
        &self,
        paths: &Paths,
        record: &BoundRuntimeRecord,
        phase: SessionAuthorityPhase,
    ) -> Result<()> {
        confirm_session_authority(&self.guardians, paths, record, phase)
    }
}
