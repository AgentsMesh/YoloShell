use super::super::ingress_task::settle_intake;
use anyhow::Result;
use tokio::task::JoinSet;

const INNER_FAILURE: &str = "intake inner failure";
const PANIC_DETAIL: &str = "intake panic";

#[tokio::test]
async fn intake_settlement_accepts_a_successful_task() {
    let mut tasks = JoinSet::new();
    tasks.spawn(async { Ok(()) });
    assert!(settle_intake(tasks.join_next().await).is_ok());
}

#[tokio::test]
async fn intake_settlement_rejects_missing_and_failed_tasks() {
    assert!(settle_intake(None).is_err());
    let mut failed = JoinSet::new();
    failed.spawn(async { anyhow::bail!(INNER_FAILURE) });
    assert!(settle_intake(failed.join_next().await).is_err());
}

#[tokio::test]
async fn intake_settlement_contains_a_panicking_task() {
    let mut tasks = JoinSet::<Result<()>>::new();
    tasks.spawn(async { panic!("{PANIC_DETAIL}") });
    assert!(settle_intake(tasks.join_next().await).is_err());
}
