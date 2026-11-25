use anyhow::Result;
use std::sync::Arc;
use yolo_broker_supervision_state::{ChildLaunch, CredentialSource};
use yolo_remote_broker::{
    AuthorizedBrokerStartup, BrokerConfig, BrokerConnectionFactory, BrokerPaths, RemoteBroker,
};

pub async fn serve(launch: ChildLaunch, credentials: CredentialSource) -> Result<()> {
    prepare_server(launch, credentials).await?.serve().await
}

pub async fn prepare_server(
    launch: ChildLaunch,
    credentials: CredentialSource,
) -> Result<RemoteBroker> {
    prepare(launch, credentials, None).await
}

#[cfg(feature = "yolo-test-support")]
pub async fn prepare_server_with_factory(
    launch: ChildLaunch,
    credentials: CredentialSource,
    factory: Arc<dyn BrokerConnectionFactory>,
) -> Result<RemoteBroker> {
    prepare(launch, credentials, Some(factory)).await
}

#[cfg(feature = "yolo-test-support")]
pub async fn serve_with_factory(
    launch: ChildLaunch,
    credentials: CredentialSource,
    factory: Arc<dyn BrokerConnectionFactory>,
) -> Result<()> {
    prepare_server_with_factory(launch, credentials, factory)
        .await?
        .serve()
        .await
}

async fn prepare(
    launch: ChildLaunch,
    credentials: CredentialSource,
    factory: Option<Arc<dyn BrokerConnectionFactory>>,
) -> Result<RemoteBroker> {
    let paths = BrokerPaths::new_expected(launch.state_dir().to_path_buf(), launch.state_anchor())?;
    paths.verify_state()?;
    let deadline = launch.startup_deadline().to_instant()?;
    let startup =
        AuthorizedBrokerStartup::acquire_for_launch(paths, launch.authority(), deadline).await?;
    let passphrases = crate::credentials::read(credentials)?;
    startup.verify()?;
    let config = BrokerConfig::load_expected_with_passphrases(
        launch.config(),
        launch.config_revision(),
        &passphrases,
    )?;
    startup.verify()?;
    Ok(match factory {
        Some(factory) => RemoteBroker::with_authorized_factory(config, startup, factory),
        None => RemoteBroker::new_authorized(config, startup),
    })
}
