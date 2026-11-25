use crate::LocalBackend;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;
use yolo_hub_connector_protocol::{
    Activation, ConnectorCredential, ConnectorError, ConnectorScope, Enrollment, EnrollmentSummary,
    Publication, PublicationAck,
};
use yolo_hub_connector_registry::Registry;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::{ConnectorId, ConnectorInstanceId};
use yolo_hub_terminal_protocol::PublicError;
use yolo_paths::directory::PrivateDirectory;

impl LocalBackend {
    pub fn with_connectors(
        mut self,
        directory: PrivateDirectory,
        identity: PeerIdentity,
    ) -> Result<Self, PublicError> {
        if identity.hub() != self.hub {
            return Err(PublicError::OriginMismatch);
        }
        let registry = Registry::open(directory, identity).map_err(|_| PublicError::Internal)?;
        self.connectors = Some(Arc::new(Mutex::new(registry)));
        Ok(self)
    }
    pub async fn register_connector(
        &self,
        request_id: Uuid,
        scope: ConnectorScope,
    ) -> Result<Enrollment, ConnectorError> {
        if request_id.is_nil() {
            return Err(ConnectorError::InvalidRequest);
        }
        scope.validate_for(self.hub)?;
        let _observation = self.connector_observation().await;
        let mut registry = self.connector_registry().await?;
        let admission = self.connector_admission(scope.terminal).await;
        registry.reconcile_observed(&[(scope.terminal, admission)].into_iter().collect())?;
        registry.register(request_id, scope, admission)
    }
    pub async fn activate_connector(
        &self,
        id: ConnectorId,
        credential: ConnectorCredential,
        instance: ConnectorInstanceId,
    ) -> Result<Activation, ConnectorError> {
        let _observation = self.connector_observation().await;
        let mut registry = self.connector_registry().await?;
        let scope = registry.credential_scope(id, credential)?;
        let admission = self.connector_admission(scope.terminal).await;
        registry.activate(id, credential, instance, admission)
    }
    pub async fn publish_connector(
        &self,
        activation: Activation,
        publication: Publication,
    ) -> Result<PublicationAck, ConnectorError> {
        let _observation = self.connector_observation().await;
        let mut registry = self.connector_registry().await?;
        let scope = registry.active_scope(activation)?;
        publication.validate()?;
        if publication.connector_id != activation.connector_id
            || publication.generation != activation.generation
            || publication.agent != activation.agent
            || publication.terminal != activation.terminal
        {
            return Err(ConnectorError::StaleGeneration);
        }
        let admission = self.connector_admission(scope.terminal).await;
        registry.publish(
            activation,
            publication,
            admission,
            epoch_millis(SystemTime::now())?,
        )
    }
    pub async fn revoke_connector(
        &self,
        request_id: Uuid,
        id: ConnectorId,
        expected_revision: u64,
    ) -> Result<EnrollmentSummary, ConnectorError> {
        self.connector_registry()
            .await?
            .revoke(request_id, id, expected_revision)
    }
    pub async fn list_connectors(&self) -> Result<Vec<EnrollmentSummary>, ConnectorError> {
        self.connector_registry().await?.list()
    }
    pub async fn disconnect_connector(&self, activation: Activation) -> Result<(), ConnectorError> {
        self.connector_registry().await?.disconnect(activation)
    }
    pub async fn disconnect_all_connectors(&self) -> Result<(), ConnectorError> {
        self.connector_registry().await?.disconnect_all()
    }
    async fn connector_registry(&self) -> Result<MutexGuard<'_, Registry>, ConnectorError> {
        Ok(self
            .connectors
            .as_ref()
            .ok_or(ConnectorError::Unavailable)?
            .lock()
            .await)
    }
}

pub(crate) fn epoch_millis(time: SystemTime) -> Result<u64, ConnectorError> {
    time.duration_since(UNIX_EPOCH)
        .map_err(|_| ConnectorError::Internal)?
        .as_millis()
        .try_into()
        .map_err(|_| ConnectorError::Internal)
}
