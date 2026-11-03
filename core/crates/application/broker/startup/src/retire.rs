use super::RetireRequest;
use anyhow::Result;
use std::time::Duration;
use yolo_broker_profile_authority::RetirementAuthorityGuard;
use yolo_remote_broker::{BrokerPaths, retire_broker};

const BROKER_RETIREMENT_TIMEOUT: Duration = Duration::from_secs(6);

pub async fn run(options: RetireRequest) -> Result<()> {
    let authority = RetirementAuthorityGuard::acquire(
        &options.authority_document,
        &options.authority_lock,
        &options.expected_host,
        options.expected_mutation_id,
    )?;
    let paths = BrokerPaths::new(options.broker_state)?;
    let deadline = tokio::time::Instant::now() + BROKER_RETIREMENT_TIMEOUT;
    let result = retire_broker(&paths, &options.expected_host, deadline).await;
    drop(authority);
    result
}
