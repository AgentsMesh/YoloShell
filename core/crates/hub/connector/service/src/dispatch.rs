use crate::role::Role;
use yolo_hub_connector_protocol::{Command, ConnectorError, Reply, Request};
use yolo_hub_local_backend::LocalBackend;

pub(crate) async fn dispatch(
    backend: &LocalBackend,
    role: &mut Role,
    request: &Request,
) -> Result<Reply, ConnectorError> {
    role.authorize(&request.command)?;
    match &request.command {
        Command::Hello {} => Ok(Reply::Hello {
            hub: backend.hub_id(),
        }),
        Command::List {} => Ok(Reply::Enrollments {
            enrollments: backend.list_connectors().await?,
        }),
        Command::Register { scope } => Ok(Reply::Enrolled {
            enrollment: backend
                .register_connector(request.request_id, scope.clone())
                .await?,
        }),
        Command::Activate {
            connector_id,
            credential,
            instance_id,
        } => {
            let activation = backend
                .activate_connector(*connector_id, *credential, *instance_id)
                .await?;
            role.activation = Some(activation);
            Ok(Reply::Activated { activation })
        }
        Command::Publish { publication } => Ok(Reply::Published {
            ack: backend
                .publish_connector(
                    role.activation.ok_or(ConnectorError::Unauthorized)?,
                    publication.clone(),
                )
                .await?,
        }),
        Command::Revoke {
            connector_id,
            expected_revision,
        } => Ok(Reply::Revoked {
            enrollment: backend
                .revoke_connector(request.request_id, *connector_id, *expected_revision)
                .await?,
        }),
    }
}
