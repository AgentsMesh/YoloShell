use super::*;
use yolo_remote_provisioning_protocol::{BrokerProvisionCall, ProvisionStatus};

impl BrokerClient {
    pub async fn provision_hub(
        &self,
        call: BrokerProvisionCall,
        deadline: tokio::time::Instant,
    ) -> Result<ProvisionStatus, BrokerClientError> {
        self.provision_request(BrokerRequest::ProvisionHub(call.clone()), &call, deadline)
            .await
    }
    pub async fn recover_hub_provision(
        &self,
        call: BrokerProvisionCall,
        deadline: tokio::time::Instant,
    ) -> Result<ProvisionStatus, BrokerClientError> {
        self.provision_request(
            BrokerRequest::RecoverHubProvision(call.clone()),
            &call,
            deadline,
        )
        .await
    }
    pub async fn advance_hub_provision(
        &self,
        call: BrokerProvisionCall,
        deadline: tokio::time::Instant,
    ) -> Result<ProvisionStatus, BrokerClientError> {
        self.provision_request(
            BrokerRequest::AdvanceHubProvision(call.clone()),
            &call,
            deadline,
        )
        .await
    }
    async fn provision_request(
        &self,
        request: BrokerRequest,
        call: &BrokerProvisionCall,
        deadline: tokio::time::Instant,
    ) -> Result<ProvisionStatus, BrokerClientError> {
        call.validate()
            .map_err(|_| BrokerClientError::RequestMismatch)?;
        let mut connection = self.connect(None, deadline).await?;
        let response = request_on(connection.stream(), request, deadline).await?;
        let BrokerResult::HubProvision(status) = response else {
            return Err(BrokerClientError::UnexpectedResult);
        };
        status
            .validate_for(&call.request)
            .map_err(|_| BrokerClientError::RequestMismatch)?;
        connection.verify_path()?;
        Ok(status)
    }
}
