pub(super) const FORCE_SETTLEMENT_INTEGRATION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RootExitCleanupPath {
    InlineComplete,
    FaultReaperDeferred,
}
