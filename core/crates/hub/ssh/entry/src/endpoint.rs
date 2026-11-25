use crate::{
    EntryProfile, ProfileError, connect, connection::Connection, constants::*, error,
    lifecycle::Lifecycle,
};
use russh::keys::{PrivateKey, PublicKey};
use std::{fmt, future::Future, io::ErrorKind, pin::Pin, sync::Arc};
use tokio::{
    sync::Mutex,
    time::{Instant, timeout_at},
};
use uuid::Uuid;
use yolo_hub_client::{ClientError, EntryConnector, HubEntryStream};
use yolo_hub_identity::HubId;

/// Shares one outer SSH account connection and one authenticated inner Hub connection.
pub struct SshEntry {
    profile: EntryProfile,
    pin: PublicKey,
    outer_key: Arc<PrivateKey>,
    inner_key: Arc<PrivateKey>,
    cached: Mutex<Option<Arc<Connection>>>,
    lifecycle: Lifecycle,
}
impl SshEntry {
    pub fn new(
        profile: EntryProfile,
        outer_key: Arc<PrivateKey>,
        inner_key: Arc<PrivateKey>,
    ) -> Result<Self, ProfileError> {
        profile.validate()?;
        if outer_key.is_encrypted()
            || inner_key.is_encrypted()
            || inner_key.public_key().key_data().ed25519().is_none()
        {
            return Err(ProfileError::Credential);
        }
        let pin = profile.pinned_key()?;
        Ok(Self {
            profile,
            pin,
            outer_key,
            inner_key,
            cached: Mutex::new(None),
            lifecycle: Lifecycle::default(),
        })
    }
    pub fn profile(&self) -> &EntryProfile {
        &self.profile
    }
    pub async fn close(&self, deadline: Instant) -> Result<(), ClientError> {
        self.lifecycle.close();
        let cached = timeout_at(deadline, self.cached.lock())
            .await
            .map_err(error::timeout)?
            .take();
        if let Some(connection) = cached {
            connection.close(deadline).await?;
        }
        Ok(())
    }
    async fn connection(&self, deadline: Instant) -> Result<Arc<Connection>, ClientError> {
        let mut cached = timeout_at(deadline, self.cached.lock())
            .await
            .map_err(error::timeout)?;
        if self.lifecycle.is_closed() {
            return Err(error::transport(ErrorKind::NotConnected, ENTRY_CLOSED));
        }
        if cached
            .as_ref()
            .is_some_and(|connection| connection.is_closed())
        {
            cached.take();
        }
        if let Some(connection) = cached.as_ref() {
            return Ok(connection.clone());
        }
        let connecting = connect::connect(
            &self.profile,
            self.pin.clone(),
            self.outer_key.clone(),
            self.inner_key.clone(),
            &self.lifecycle,
            deadline,
        );
        let connection = tokio::select! {
            result = connecting => Arc::new(result?),
            _ = self.lifecycle.closed() => return Err(error::transport(ErrorKind::NotConnected, ENTRY_CLOSED)),
        };
        if self.lifecycle.is_closed() {
            return Err(error::transport(ErrorKind::NotConnected, ENTRY_CLOSED));
        }
        *cached = Some(connection.clone());
        Ok(connection)
    }
}
impl EntryConnector for SshEntry {
    fn validate(&self) -> Result<(), ClientError> {
        self.profile
            .validate()
            .map_err(|_| ClientError::InvalidEndpoint)
    }
    fn client_id(&self) -> Uuid {
        self.profile.client_id
    }
    fn open(
        &self,
        expected: HubId,
        deadline: Instant,
    ) -> Pin<Box<dyn Future<Output = Result<HubEntryStream, ClientError>> + Send + '_>> {
        Box::pin(async move {
            self.validate()?;
            if expected != self.profile.hub.hub_id {
                return Err(ClientError::InvalidEndpoint);
            }
            self.connection(deadline).await?.open(deadline).await
        })
    }
}
impl Drop for SshEntry {
    fn drop(&mut self) {
        self.lifecycle.close();
        if let Some(connection) = self.cached.get_mut().take() {
            connection.close_now();
        }
    }
}
impl fmt::Debug for SshEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SshEntry")
            .field("profile", &self.profile)
            .finish_non_exhaustive()
    }
}
