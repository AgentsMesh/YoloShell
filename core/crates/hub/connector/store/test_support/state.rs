use super::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_hub_connector_store::{Seed, StoredState};
use yolo_hub_identity::AgentAddress;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub agent: AgentAddress,
    pub revision: u64,
    pub revoked: bool,
    pub payload: String,
}
impl State {
    pub fn initial(seed: &Seed) -> Self {
        Self {
            agent: seed.agent,
            revision: INITIAL_REVISION,
            revoked: false,
            payload: String::new(),
        }
    }
    pub fn revoked(seed: &Seed) -> Self {
        Self {
            revision: NEXT_REVISION,
            revoked: true,
            ..Self::initial(seed)
        }
    }
}
impl StoredState for State {
    fn validate(&self, seed: &Seed) -> Result<()> {
        ensure!(
            self.agent == seed.agent && self.revision >= INITIAL_REVISION,
            STATE_INVALID
        );
        Ok(())
    }
}
