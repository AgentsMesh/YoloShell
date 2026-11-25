use crate::ConnectorError;
use serde::{Deserialize, Serialize};
use yolo_hub_agent_protocol::{Activity, AgentWorkingDirectory, Lifecycle};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "StateWire")]
pub struct ReportedState {
    pub lifecycle: Lifecycle,
    pub activity: Activity,
    pub working_directory: Option<AgentWorkingDirectory>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StateWire {
    lifecycle: Lifecycle,
    activity: Activity,
    working_directory: Option<AgentWorkingDirectory>,
}

impl ReportedState {
    pub fn validate(&self) -> Result<(), ConnectorError> {
        if matches!(self.lifecycle, Lifecycle::Exited { .. })
            && !matches!(self.activity, Activity::Unknown | Activity::Idle)
        {
            return Err(ConnectorError::InvalidRequest);
        }
        Ok(())
    }
}

impl TryFrom<StateWire> for ReportedState {
    type Error = ConnectorError;
    fn try_from(wire: StateWire) -> Result<Self, Self::Error> {
        let value = Self {
            lifecycle: wire.lifecycle,
            activity: wire.activity,
            working_directory: wire.working_directory,
        };
        value.validate()?;
        Ok(value)
    }
}
