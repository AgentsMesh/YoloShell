use anyhow::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Checkpoint {
    BeforeRecordCommit,
    AfterRecordCommit,
    AfterIndexCommit,
}
pub(crate) fn reach(point: Checkpoint) -> Result<()> {
    #[cfg(test)]
    return crate::test_hooks::reach(point);
    #[cfg(not(test))]
    {
        let _ = point;
        Ok(())
    }
}
