use anyhow::Result;
use yolo_remote_broker::{BrokerConfigGeneration, BrokerStatus};
const OPTIONS_MISMATCH: &str = "Hub broker start options differ from launch permit";

pub(super) trait StartAuthority: Send + Sync {
    fn verify(&self) -> Result<()>;
    fn verify_request(&self, request: &crate::StartRequest) -> Result<()>;
    fn verify_generation(&self, generation: &BrokerConfigGeneration) -> Result<()>;
    fn commit(&self, _ready: &BrokerStatus) -> Result<()> {
        Ok(())
    }
    fn finished(&self, _ready: &BrokerStatus) -> Result<()> {
        Ok(())
    }
}

impl StartAuthority for yolo_broker_profile_authority::StartAuthorityGuard {
    fn verify(&self) -> Result<()> {
        self.verify()
    }
    fn verify_request(&self, _: &crate::StartRequest) -> Result<()> {
        self.verify()
    }
    fn verify_generation(&self, generation: &BrokerConfigGeneration) -> Result<()> {
        self.verify_config(generation.host_id(), generation.profile_authority_sha256())
    }
}

impl StartAuthority for yolo_broker_launch_authority::HubStartAuthorityGuard {
    fn verify(&self) -> Result<()> {
        self.verify()
    }
    fn verify_request(&self, request: &crate::StartRequest) -> Result<()> {
        self.verify()?;
        anyhow::ensure!(
            request.config_source == self.scope().config_source
                && request.broker_state == self.scope().broker_state,
            OPTIONS_MISMATCH
        );
        Ok(())
    }
    fn verify_generation(&self, generation: &BrokerConfigGeneration) -> Result<()> {
        self.verify_generation(generation)
    }
    fn commit(&self, ready: &BrokerStatus) -> Result<()> {
        self.commit(ready)
    }
    fn finished(&self, ready: &BrokerStatus) -> Result<()> {
        self.finished(ready)
    }
}
