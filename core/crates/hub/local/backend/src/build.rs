use crate::{LocalBackend, errors::map_error};
use std::sync::Arc;
use yolo_executable_identity::CapturedExecutable;
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_dispatch::{BuildFuture, BuildProvider};
use yolo_hub_file_executor::FileExecutor;
use yolo_hub_file_helper_protocol::MAX_OPERATION_TIME;
use yolo_hub_terminal_protocol::PublicError;

struct LiveBuild {
    hub: yolo_hub_identity::HubId,
    client: yolo_client::Client,
    helper: FileExecutor,
    image: CapturedExecutable,
}

impl LocalBackend {
    pub fn build_provider(&self) -> Result<Arc<dyn BuildProvider>, PublicError> {
        let program = self.file_helper.clone().ok_or(PublicError::Unsupported)?;
        Ok(Arc::new(LiveBuild {
            hub: self.hub,
            client: self.client.clone(),
            helper: FileExecutor::new(program)?,
            image: CapturedExecutable::capture().map_err(|_| PublicError::Internal)?,
        }))
    }
}

impl BuildProvider for LiveBuild {
    fn build(&self) -> BuildFuture<'_> {
        Box::pin(async move {
            let hub = self
                .image
                .image()
                .map_err(|_| PublicError::Internal)?
                .clone();
            let host = self
                .client
                .build_image()
                .await
                .map_err(|error| match error {
                    yolo_client::ClientError::HostCapabilityUnavailable { .. } => {
                        PublicError::Unsupported
                    }
                    error => map_error(error),
                })?;
            let deadline = tokio::time::Instant::now() + MAX_OPERATION_TIME;
            let file_helper = self.helper.build_image(self.hub, deadline).await?;
            self.image.verify().map_err(|_| PublicError::Internal)?;
            Ok(ServiceBuild {
                host,
                hub,
                file_helper,
            })
        })
    }
}
