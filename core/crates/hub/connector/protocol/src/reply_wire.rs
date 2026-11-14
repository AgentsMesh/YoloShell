use crate::{Activation, ConnectorError, Enrollment, EnrollmentSummary, PublicationAck, Reply};
use serde::Deserialize;
use yolo_hub_identity::HubId;

#[derive(Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ReplyWire {
    Hello { hub: HubId },
    Enrolled { enrollment: Enrollment },
    Enrollments { enrollments: Vec<EnrollmentSummary> },
    Activated { activation: Activation },
    Published { ack: PublicationAck },
    Revoked { enrollment: EnrollmentSummary },
    Error { code: ConnectorError },
}

impl TryFrom<ReplyWire> for Reply {
    type Error = ConnectorError;
    fn try_from(wire: ReplyWire) -> Result<Self, ConnectorError> {
        let value = match wire {
            ReplyWire::Hello { hub } => Self::Hello { hub },
            ReplyWire::Enrolled { enrollment } => Self::Enrolled { enrollment },
            ReplyWire::Enrollments { enrollments } => Self::Enrollments { enrollments },
            ReplyWire::Activated { activation } => Self::Activated { activation },
            ReplyWire::Published { ack } => Self::Published { ack },
            ReplyWire::Revoked { enrollment } => Self::Revoked { enrollment },
            ReplyWire::Error { code } => Self::Error { code },
        };
        value.validate()?;
        Ok(value)
    }
}
