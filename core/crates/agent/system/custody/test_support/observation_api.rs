#[doc(hidden)]
pub fn guardians_for_test(
    path: &Path,
    invocation_id: Uuid,
) -> Result<[crate::process::ProcessIdentity; types::GUARDIAN_COUNT]> {
    persistence::update(path, |ledger| {
        let record = ledger
            .records
            .iter()
            .find(|record| record.spec.invocation_id == invocation_id)
            .context("Claude custody record is unavailable")?;
        Ok(record
            .spec
            .guardians
            .clone()
            .map(|guardian| crate::process::ProcessIdentity {
                pid: guardian.pid,
                start_time: guardian.start_time,
            }))
    })
}

#[doc(hidden)]
pub fn is_indeterminate_for_test(path: &Path, invocation_id: Uuid) -> Result<bool> {
    persistence::update(path, |ledger| {
        let record = ledger
            .records
            .iter()
            .find(|record| record.spec.invocation_id == invocation_id)
            .context("Claude custody record is unavailable")?;
        Ok(record.state == types::CustodyState::Indeterminate)
    })
}

#[doc(hidden)]
pub fn is_active_for_test(path: &Path, invocation_id: Uuid) -> Result<bool> {
    persistence::update(path, |ledger| {
        let record = ledger
            .records
            .iter()
            .find(|record| record.spec.invocation_id == invocation_id)
            .context("Claude custody record is unavailable")?;
        Ok(record.publication == types::CustodyPublication::Active)
    })
}
