#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Checkpoint {
    BeforeCommit,
    CommitObserved,
    WriteAcknowledged,
}

pub(crate) fn reach(point: Checkpoint) -> anyhow::Result<()> {
    #[cfg(test)]
    return crate::test_hooks::reach(point);
    #[cfg(not(test))]
    {
        let _ = point;
        Ok(())
    }
}
