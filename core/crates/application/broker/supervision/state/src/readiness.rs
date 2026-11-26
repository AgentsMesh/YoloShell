/// Observed child data; publication and adoption still require the supervision protocol.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerReadiness {
    pub instance_id: uuid::Uuid,
    pub host_id: String,
    pub config_revision: String,
}
