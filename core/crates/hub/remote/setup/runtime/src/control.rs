use crate::{Engine, constants::*};
use anyhow::{Result, bail};
use yolo_hub_dispatch::Operation;
use yolo_hub_management_protocol::ConfigurationSnapshot;
use yolo_hub_rpc_protocol::{Command, Reply, Request};

impl Engine {
    pub async fn configuration(&self) -> Result<ConfigurationSnapshot> {
        self.state.verify()?;
        let request = Request::new(self.state.hub_id(), Command::GetConfiguration {})?;
        match self.inner.dispatch(request).await? {
            Operation::Reply(Reply::Configuration { snapshot }) => {
                snapshot.validate()?;
                anyhow::ensure!(
                    snapshot.state.identity == self.state.peer_identity(),
                    INVALID
                );
                self.state.verify()?;
                Ok(snapshot)
            }
            _ => bail!(UNEXPECTED),
        }
    }
}
