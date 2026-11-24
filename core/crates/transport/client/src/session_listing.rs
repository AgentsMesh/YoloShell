use super::*;

impl Client {
    pub async fn list_sessions(
        &self,
        include_archived: bool,
    ) -> Result<Vec<TerminalView>, ClientError> {
        self.list_sessions_with_completeness(include_archived, false)
            .await
    }

    pub async fn list_sessions_complete(
        &self,
        include_archived: bool,
    ) -> Result<Vec<TerminalView>, ClientError> {
        self.list_sessions_with_completeness(include_archived, true)
            .await
    }

    async fn list_sessions_with_completeness(
        &self,
        include_archived: bool,
        require_complete: bool,
    ) -> Result<Vec<TerminalView>, ClientError> {
        let result: ListSessionsResult = self
            .request(
                method::LIST_SESSIONS,
                &ListSessionsRequest {
                    include_archived,
                    require_complete,
                },
            )
            .await?;
        if require_complete && !result.complete {
            return Err(ClientError::IncompleteCatalogInventory);
        }
        Ok(result.sessions)
    }
}
