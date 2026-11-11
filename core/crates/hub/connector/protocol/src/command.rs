use crate::{ConnectorCredential, ConnectorError, ConnectorScope, Publication, validation};
use serde::{Deserialize, Serialize};
use yolo_hub_identity::{ConnectorId, ConnectorInstanceId, HubId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Hello {},
    List {},
    Register {
        scope: ConnectorScope,
    },
    Activate {
        connector_id: ConnectorId,
        credential: ConnectorCredential,
        instance_id: ConnectorInstanceId,
    },
    Publish {
        publication: Publication,
    },
    Revoke {
        connector_id: ConnectorId,
        #[serde(deserialize_with = "validation::deserialize_revision")]
        expected_revision: u64,
    },
}

impl Command {
    pub fn mutates(&self) -> bool {
        !matches!(self, Self::Hello {} | Self::List {})
    }

    pub fn validate_for(&self, hub: HubId) -> Result<(), ConnectorError> {
        match self {
            Self::Register { scope } => scope.validate_for(hub),
            Self::Publish { publication } => {
                publication.validate()?;
                if publication.agent.address.hub_id == hub {
                    Ok(())
                } else {
                    Err(ConnectorError::InvalidRequest)
                }
            }
            Self::Revoke {
                expected_revision, ..
            } => validation::revision(*expected_revision),
            Self::Hello {} | Self::List {} | Self::Activate { .. } => Ok(()),
        }
    }
}
