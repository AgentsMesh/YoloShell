use yolo_hub_connector_protocol::{Activation, Command, ConnectorError};

#[derive(Default)]
pub(crate) struct Role {
    pub greeted: bool,
    pub activation: Option<Activation>,
}
impl Role {
    pub fn authorize(&self, command: &Command) -> Result<(), ConnectorError> {
        match (self.activation, command) {
            (_, Command::Hello {}) => Ok(()),
            (Some(active), Command::Publish { publication })
                if publication.connector_id == active.connector_id
                    && publication.generation == active.generation
                    && publication.agent == active.agent
                    && publication.terminal == active.terminal =>
            {
                Ok(())
            }
            (Some(_), Command::Publish { .. }) => Err(ConnectorError::StaleGeneration),
            (None, Command::Publish { .. }) | (Some(_), _) => Err(ConnectorError::Unauthorized),
            (None, _) => Ok(()),
        }
    }
}
