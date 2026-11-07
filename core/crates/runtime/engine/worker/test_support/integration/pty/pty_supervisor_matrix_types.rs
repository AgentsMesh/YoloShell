#[derive(Clone, Copy)]
enum SupervisorMatrixStep {
    ChildExited,
    Cancelled,
    Eof { output: bool, child_waited: bool },
    Failure { child_waited: bool },
    FailureWithPending,
    FailureWithPendingCancellation,
    FailureWithPendingNamespaceLoss,
    FailureWithNamespaceLoss,
}
