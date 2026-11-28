use super::*;
use yolo_build_image::BuildImage;

impl Client {
    /// Query the image on the same connection that authenticated the Host.
    pub async fn build_image(&self) -> Result<BuildImage, ClientError> {
        let deadline = self.operation_deadline();
        let mut connection = self.connect_until(deadline).await?;
        let status: HostStatus = match connection.take_authenticated_status() {
            Some(status) => status,
            None => {
                connection
                    .request_until(method::HOST_STATUS, &(), deadline)
                    .await?
            }
        };
        let capability = crate::protocol::capability::HOST_BUILD;
        if !status.capabilities.iter().any(|value| value == capability) {
            return Err(ClientError::HostCapabilityUnavailable {
                capability: capability.to_owned(),
            });
        }
        connection
            .request_until(method::HOST_BUILD, &(), deadline)
            .await
    }
}
