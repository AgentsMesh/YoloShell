use super::Fixture;
use crate::constants::*;
use std::sync::Arc;
use yolo_hub_dispatch::{Dispatcher, Operation};
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationSnapshot};
use yolo_hub_remote_setup_protocol::{
    RemoteSetupCall, RemoteSetupOutcome, RemoteSetupStatus, SetupAction,
};
use yolo_hub_rpc_protocol::{Command, Reply, Request};

impl Fixture {
    pub async fn client(&self) -> yolo_hub_client::LocalClient {
        yolo_hub_client::Client::connect_local(
            &self.state.socket_path(),
            self.state.hub_id(),
            deadline(),
        )
        .await
        .unwrap()
    }
    pub async fn action(
        &self,
        action: SetupAction,
    ) -> Result<RemoteSetupStatus, yolo_hub_terminal_protocol::PublicError> {
        let command = Command::RemoteSetup {
            call: RemoteSetupCall {
                action,
                request: self.request.clone(),
                credentials: None,
            },
        };
        let Reply::RemoteSetup { status } =
            call(&(self.runtime.clone() as Arc<dyn Dispatcher>), command).await?
        else {
            panic!()
        };
        status.validate_for(&self.request).unwrap();
        Ok(status)
    }
    pub async fn settled(&self) -> RemoteSetupStatus {
        tokio::time::timeout_at(deadline(), async {
            loop {
                let status = self.action(SetupAction::Poll).await.unwrap();
                if !matches!(status.outcome, RemoteSetupOutcome::Running {}) {
                    return status;
                }
                tokio::time::sleep(POLL_DELAY).await;
            }
        })
        .await
        .unwrap()
    }
    pub async fn configuration(&self) -> ConfigurationSnapshot {
        let Reply::Configuration { snapshot } = call(
            &(self.managed.clone() as Arc<dyn Dispatcher>),
            Command::GetConfiguration {},
        )
        .await
        .unwrap() else {
            panic!()
        };
        snapshot
    }
    pub async fn configure(&self, mut snapshot: ConfigurationSnapshot) -> ConfigurationSnapshot {
        snapshot.state.last_change = uuid::Uuid::new_v4();
        let Reply::Configuration { snapshot } = call(
            &(self.managed.clone() as Arc<dyn Dispatcher>),
            Command::Configure {
                change: ConfigurationChange {
                    request_id: snapshot.state.last_change,
                    expected_revision: snapshot.state.revision,
                    configuration: snapshot.state.configuration,
                },
            },
        )
        .await
        .unwrap() else {
            panic!()
        };
        snapshot
    }
}
pub async fn call(
    dispatcher: &Arc<dyn Dispatcher>,
    command: Command,
) -> Result<Reply, yolo_hub_terminal_protocol::PublicError> {
    let request = Request::new(dispatcher.hub_id(), command).unwrap();
    let Operation::Reply(reply) = dispatcher.dispatch(request).await? else {
        panic!()
    };
    Ok(reply)
}
