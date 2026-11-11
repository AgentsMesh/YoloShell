use crate::{
    CONNECTOR_PROTOCOL_VERSION, Command, ConnectorError, EnrollmentStatus,
    FIRST_CONNECTOR_REVISION, Reply, Request, validation,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::HubId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ResponseWire")]
pub struct Response {
    pub version: u16,
    #[serde(with = "validation::id")]
    pub request_id: Uuid,
    pub hub: HubId,
    pub reply: Reply,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseWire {
    version: u16,
    #[serde(with = "validation::id")]
    request_id: Uuid,
    hub: HubId,
    reply: Reply,
}

impl Response {
    pub fn new(request: &Request, reply: Reply) -> Self {
        Self {
            version: CONNECTOR_PROTOCOL_VERSION,
            request_id: request.request_id,
            hub: request.hub,
            reply,
        }
    }

    pub fn validate(&self) -> Result<(), ConnectorError> {
        if self.version != CONNECTOR_PROTOCOL_VERSION {
            return Err(ConnectorError::UnsupportedSchema);
        }
        validation::request_id(self.request_id)?;
        self.reply.validate_for_hub(self.hub)
    }

    pub fn validate_for(&self, request: &Request) -> Result<(), ConnectorError> {
        request.validate()?;
        self.validate()?;
        if self.request_id != request.request_id || self.hub != request.hub {
            return Err(ConnectorError::InvalidRequest);
        }
        let matches = match (&self.reply, &request.command) {
            (Reply::Error { .. }, _) => true,
            (Reply::Hello { hub }, Command::Hello {}) => *hub == request.hub,
            (Reply::Enrollments { .. }, Command::List {}) => true,
            (Reply::Enrolled { enrollment }, Command::Register { scope }) => {
                enrollment.scope == *scope
            }
            (Reply::Activated { activation }, Command::Activate { connector_id, .. }) => {
                activation.connector_id == *connector_id
            }
            (Reply::Published { ack }, Command::Publish { publication }) => {
                ack.connector_id == publication.connector_id
                    && ack.generation == publication.generation
                    && ack.sequence == publication.sequence
                    && ack.cursor.target() == publication.agent
            }
            (
                Reply::Revoked { enrollment },
                Command::Revoke {
                    connector_id,
                    expected_revision,
                },
            ) => {
                enrollment.connector_id == *connector_id
                    && enrollment.status == EnrollmentStatus::Revoked
                    && expected_revision.checked_add(FIRST_CONNECTOR_REVISION)
                        == Some(enrollment.revision)
            }
            _ => false,
        };
        if matches {
            Ok(())
        } else {
            Err(ConnectorError::InvalidRequest)
        }
    }
}

impl TryFrom<ResponseWire> for Response {
    type Error = ConnectorError;
    fn try_from(wire: ResponseWire) -> Result<Self, Self::Error> {
        let value = Self {
            version: wire.version,
            request_id: wire.request_id,
            hub: wire.hub,
            reply: wire.reply,
        };
        value.validate()?;
        Ok(value)
    }
}
