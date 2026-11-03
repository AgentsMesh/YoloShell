use crate::{
    Activation, ConnectorError, Enrollment, EnrollmentSummary, MAX_CONNECTORS, PublicationAck,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use yolo_hub_identity::HubId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "result",
    rename_all = "snake_case",
    try_from = "crate::reply_wire::ReplyWire"
)]
pub enum Reply {
    Hello { hub: HubId },
    Enrolled { enrollment: Enrollment },
    Enrollments { enrollments: Vec<EnrollmentSummary> },
    Activated { activation: Activation },
    Published { ack: PublicationAck },
    Revoked { enrollment: EnrollmentSummary },
    Error { code: ConnectorError },
}

impl Reply {
    pub fn validate(&self) -> Result<(), ConnectorError> {
        match self {
            Self::Enrolled { enrollment } => enrollment.validate(),
            Self::Revoked { enrollment } => {
                enrollment.validate()?;
                if enrollment.status != crate::EnrollmentStatus::Revoked {
                    return Err(ConnectorError::InvalidRequest);
                }
                Ok(())
            }
            Self::Activated { activation } => activation.validate(),
            Self::Published { ack } => ack.validate(),
            Self::Enrollments { enrollments } => {
                if enrollments.len() > MAX_CONNECTORS {
                    return Err(ConnectorError::Capacity);
                }
                let mut unique = BTreeSet::new();
                for enrollment in enrollments {
                    enrollment.validate()?;
                    if !unique.insert(enrollment.connector_id) {
                        return Err(ConnectorError::InvalidRequest);
                    }
                }
                Ok(())
            }
            Self::Hello { .. } | Self::Error { .. } => Ok(()),
        }
    }

    pub fn validate_for_hub(&self, hub: HubId) -> Result<(), ConnectorError> {
        self.validate()?;
        let origin = match self {
            Self::Hello { hub } => Some(*hub),
            Self::Enrolled { enrollment } => Some(enrollment.agent.hub_id),
            Self::Revoked { enrollment } => Some(enrollment.agent.hub_id),
            Self::Activated { activation } => Some(activation.agent.address.hub_id),
            Self::Published { ack } => Some(ack.cursor.target().address.hub_id),
            Self::Enrollments { enrollments } => {
                for enrollment in enrollments {
                    if enrollment.agent.hub_id != hub {
                        return Err(ConnectorError::InvalidRequest);
                    }
                }
                None
            }
            Self::Error { .. } => None,
        };
        if origin.is_some_and(|origin| origin != hub) {
            return Err(ConnectorError::InvalidRequest);
        }
        Ok(())
    }
}
