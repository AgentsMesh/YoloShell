use super::supervision_activation::{ActivationReceiver, BrokerReadiness};
use anyhow::Result;
use yolo_broker_supervision_state::{ChildLaunch, CredentialSource};

pub async fn run_supervised(
    launch: ChildLaunch,
    credentials: CredentialSource,
    activation_fd: std::os::fd::RawFd,
) -> Result<()> {
    let deadline = launch.startup_deadline().to_instant()?.into_std();
    let control = ActivationReceiver::inherit(activation_fd)?.await_activation()?;
    let readiness = control.readiness_publisher()?;
    let signals = control.monitor_supervisor(deadline)?;
    let mut lost = signals.lost;
    let broker = tokio::select! {
        result = crate::prepare_server(launch, credentials) => result?,
        _ = &mut lost => anyhow::bail!("broker supervisor exited during preparation"),
    };
    let status = broker.pool().status().await;
    readiness.publish(&BrokerReadiness {
        instance_id: status.instance_id,
        host_id: status.host_id,
        config_revision: status.config_revision,
    })?;
    broker
        .serve_supervised_until(signals.adoption, async move {
            let _ = lost.await;
        })
        .await
}
