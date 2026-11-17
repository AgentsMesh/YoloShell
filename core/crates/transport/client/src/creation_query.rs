use super::*;

impl Client {
    pub async fn query_creation(
        &self,
        request: &CreateTerminal,
    ) -> Result<crate::model::CreationQueryResult, ClientError> {
        let result: crate::model::CreationQueryResult =
            self.request(method::QUERY_CREATION, request).await?;
        if !result.matches_request(request.creation_token) {
            return Err(ProtocolError::ResponseMismatch.into());
        }
        Ok(result)
    }
}
