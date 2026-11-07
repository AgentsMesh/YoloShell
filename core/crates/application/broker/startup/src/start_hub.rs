use super::*;
use yolo_broker_launch_authority::HubStartAuthorityGuard;
pub async fn run() -> Result<BrokerStatus> {
    let (authority, passphrases) =
        tokio::task::spawn_blocking(HubStartAuthorityGuard::from_stdio).await??;
    let deadline = deadline::ReconcileDeadline::until(authority.scope().deadline()?)?;
    let has_credentials = authority.has_credentials();
    let request = StartRequest {
        config_source: authority.scope().config_source.clone(),
        broker_state: authority.scope().broker_state.clone(),
    };
    let credentials = if has_credentials {
        CredentialSource::PrivateStdin
    } else {
        CredentialSource::None
    };
    let spawner = DetachedSpawner::current(passphrases, credentials)?;
    let result = reconcile_with_passphrases(
        request,
        &spawner,
        &deadline,
        spawner.passphrases(),
        Some(&authority),
    )
    .await;
    finish_after_termination_delivery(result, deadline.exit_delivery())
}
