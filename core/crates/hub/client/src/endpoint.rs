use crate::constants::{LOCAL_ENTRY_SCHEME, SSH_ENTRY_SCHEME};
use crate::{
    Client, ClientError, EndpointClient, EntryConnector, HubEntryStream,
    local::connect_local_stream,
};
use std::{os::unix::ffi::OsStrExt, path::PathBuf, str::FromStr, sync::Arc};
use tokio::time::Instant;
use yolo_hub_identity::HubId;

/// An entry transport implemented by this client. SSH parsing belongs to its
/// authenticated adapter; an unsupported SSH scheme never becomes an endpoint.
#[derive(Clone)]
pub enum HubEndpoint {
    LocalUnix(PathBuf),
    Authenticated(Arc<dyn EntryConnector>),
}

impl HubEndpoint {
    pub fn local(path: impl Into<PathBuf>) -> Self {
        Self::LocalUnix(path.into())
    }
    pub fn remote(connector: Arc<dyn EntryConnector>) -> Self {
        Self::Authenticated(connector)
    }
    pub fn validate(&self) -> Result<(), ClientError> {
        let path = match self {
            Self::LocalUnix(path) => path,
            Self::Authenticated(connector) => {
                if connector.client_id().is_nil() {
                    return Err(ClientError::InvalidEndpoint);
                }
                return connector.validate();
            }
        };
        let bytes = path.as_os_str().as_bytes();
        if bytes.is_empty() || bytes.iter().any(u8::is_ascii_control) {
            return Err(ClientError::InvalidEndpoint);
        }
        Ok(())
    }

    pub async fn connect(
        &self,
        expected: HubId,
        deadline: Instant,
    ) -> Result<EndpointClient, ClientError> {
        self.validate()?;
        let stream = match self {
            Self::LocalUnix(path) => {
                HubEntryStream::Local(connect_local_stream(path, deadline).await?)
            }
            Self::Authenticated(connector) => connector.open(expected, deadline).await?,
        };
        Client::open(stream, expected, deadline).await
    }

    pub fn local_socket(&self) -> Option<&std::path::Path> {
        match self {
            Self::LocalUnix(path) => Some(path),
            Self::Authenticated(_) => None,
        }
    }
    pub fn client_id(&self) -> Option<uuid::Uuid> {
        match self {
            Self::LocalUnix(_) => None,
            Self::Authenticated(connector) => Some(connector.client_id()),
        }
    }
}

impl PartialEq for HubEndpoint {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::LocalUnix(left), Self::LocalUnix(right)) => left == right,
            (Self::Authenticated(left), Self::Authenticated(right)) => Arc::ptr_eq(left, right),
            _ => false,
        }
    }
}
impl Eq for HubEndpoint {}
impl std::fmt::Debug for HubEndpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalUnix(path) => formatter
                .debug_tuple(crate::constants::LOCAL_ENTRY_DEBUG)
                .field(path)
                .finish(),
            Self::Authenticated(connector) => formatter
                .debug_tuple(crate::constants::REMOTE_ENTRY_DEBUG)
                .field(&connector.client_id())
                .finish(),
        }
    }
}

impl FromStr for HubEndpoint {
    type Err = ClientError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (scheme, path) = value.split_once(':').ok_or(ClientError::InvalidEndpoint)?;
        if scheme.eq_ignore_ascii_case(SSH_ENTRY_SCHEME) {
            return Err(ClientError::UnsupportedEndpoint);
        }
        if !scheme.eq_ignore_ascii_case(LOCAL_ENTRY_SCHEME) {
            return Err(ClientError::InvalidEndpoint);
        }
        // Local entries contain a literal native socket path, without URI decoding.
        let endpoint = Self::local(path);
        endpoint.validate()?;
        Ok(endpoint)
    }
}
