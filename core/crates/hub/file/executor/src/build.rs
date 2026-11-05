use crate::FileExecutor;
use tokio::time::Instant;
use uuid::Uuid;
use yolo_build_image::BuildImage;
use yolo_hub_file_helper_protocol::{Command, FILE_HELPER_SCHEMA, Payload, Request, RequestFields};
use yolo_hub_identity::HubId;
use yolo_hub_terminal_protocol::PublicError;

impl FileExecutor {
    pub async fn build_image(
        &self,
        origin: HubId,
        deadline: Instant,
    ) -> Result<BuildImage, PublicError> {
        let request = Request::new(RequestFields {
            schema_version: FILE_HELPER_SCHEMA,
            request_id: Uuid::new_v4(),
            origin,
            protected: Vec::new(),
            command: Command::Build {},
        })?;
        match self.execute(request, deadline).await? {
            Payload::Build { image } => Ok(image),
            Payload::Error { code } => Err(code),
            _ => Err(PublicError::InvalidPayload),
        }
    }
}
