use anyhow::{Context, Result, ensure};
use yolo_remote_broker::{
    BrokerClient, BrokerConfigGeneration, BrokerReconcileGuard, BrokerRetirement, BrokerStatus,
    CONNECTION_BOOTSTRAP_CAPABILITY,
};

const BOOTSTRAP_CONTEXT: &str = "bootstrap broker remote connection";
const BOOTSTRAP_IDENTITY_MISMATCH: &str = "bootstrapped broker identity changed";
const BOOTSTRAP_CONNECTION_MISSING: &str = "broker bootstrap returned no remote connection";
const BOOTSTRAP_GENERATION_INVALID: &str = "broker bootstrap returned an invalid generation";
const BOOTSTRAP_CAPABILITY_MISSING: &str = "broker bootstrap capability is missing";

pub(super) async fn require(
    observed: &BrokerRetirement,
    client: &BrokerClient,
    guard: &BrokerReconcileGuard,
    expected: &BrokerStatus,
    generation: &BrokerConfigGeneration,
    deadline: tokio::time::Instant,
) -> Result<BrokerStatus> {
    let actual = if generation.hub_transport() {
        let profile = yolo_broker_hub_protocol::BrokerHubProfile {
            host_id: generation.host_id().into(),
            profile_authority_sha256: generation.profile_authority_sha256().into(),
            config_revision: generation.revision().into(),
        };
        ensure!(
            expected.host_id == profile.host_id
                && expected.config_revision == profile.config_revision,
            BOOTSTRAP_IDENTITY_MISMATCH
        );
        let binding = observed
            .prepare_hub_transport(client, guard, profile.clone(), deadline)
            .await?;
        ensure!(
            binding.profile == profile && binding.broker_instance_id == expected.instance_id,
            BOOTSTRAP_IDENTITY_MISMATCH
        );
        let mut status = expected.clone();
        status.connection_id = Some(binding.connection_id);
        status.connection_generation = binding.connection_generation;
        status
    } else {
        observed
            .bootstrap(client, guard, deadline)
            .await
            .context(BOOTSTRAP_CONTEXT)?
    };
    ensure!(
        actual.protocol_version == expected.protocol_version
            && actual.instance_id == expected.instance_id
            && actual.host_id == expected.host_id
            && actual.config_revision == expected.config_revision,
        BOOTSTRAP_IDENTITY_MISMATCH
    );
    ensure!(
        actual.supports(if generation.hub_transport() {
            yolo_broker_hub_protocol::HUB_TRANSPORT_CAPABILITY
        } else {
            CONNECTION_BOOTSTRAP_CAPABILITY
        }),
        BOOTSTRAP_CAPABILITY_MISSING
    );
    ensure!(actual.connection_id.is_some(), BOOTSTRAP_CONNECTION_MISSING);
    ensure!(
        actual.connection_generation > u64::default(),
        BOOTSTRAP_GENERATION_INVALID
    );
    Ok(actual)
}
