use super::RemoteError;
use super::constants::{
    DEFAULT_SSH_PORT, KNOWN_HOST_PORT_PREFIX, KNOWN_HOST_PORT_SEPARATOR, MAX_REMOTE_HOST_BYTES,
    MAX_REMOTE_USER_BYTES,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RemoteEndpoint {
    user: String,
    host: String,
    port: u16,
}

impl RemoteEndpoint {
    pub fn new(
        user: impl Into<String>,
        host: impl Into<String>,
        port: u16,
    ) -> Result<Self, RemoteError> {
        let user = user.into();
        let host = host.into();
        validate_endpoint_token(&user, "user", MAX_REMOTE_USER_BYTES)?;
        validate_endpoint_token(&host, "host", MAX_REMOTE_HOST_BYTES)?;
        if user.contains('@') || host.contains(['/', '[', ']']) || port == 0 {
            return Err(RemoteError::InvalidEndpoint(
                "user, host, or port is not valid".into(),
            ));
        }
        Ok(Self {
            user,
            host: host.to_ascii_lowercase(),
            port,
        })
    }

    pub fn standard(user: impl Into<String>, host: impl Into<String>) -> Result<Self, RemoteError> {
        Self::new(user, host, DEFAULT_SSH_PORT)
    }

    pub fn user(&self) -> &str {
        &self.user
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub(crate) fn known_host_token(&self) -> String {
        if self.port == DEFAULT_SSH_PORT {
            return self.host.clone();
        }
        let host = &self.host;
        let port = self.port;
        format!("{KNOWN_HOST_PORT_PREFIX}{host}{KNOWN_HOST_PORT_SEPARATOR}{port}")
    }

    pub fn authority(&self) -> String {
        format!("{}@{}:{}", self.user, self.host, self.port)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostKey {
    algorithm: String,
    encoded: String,
}

impl HostKey {
    pub fn new(
        algorithm: impl Into<String>,
        encoded: impl Into<String>,
    ) -> Result<Self, RemoteError> {
        let algorithm = algorithm.into();
        let encoded = encoded.into();
        validate_token(&algorithm, "algorithm").map_err(invalid_key)?;
        validate_token(&encoded, "encoded key").map_err(invalid_key)?;
        Ok(Self { algorithm, encoded })
    }

    pub fn from_openssh(value: &str) -> Result<Self, RemoteError> {
        let mut fields = value.split_whitespace();
        let key = Self::new(
            fields.next().unwrap_or_default(),
            fields.next().unwrap_or_default(),
        )?;
        Ok(key)
    }

    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    pub fn encoded(&self) -> &str {
        &self.encoded
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub connection_id: uuid::Uuid,
    pub endpoint: RemoteEndpoint,
    pub host_key: HostKey,
}

fn validate_token(value: &str, label: &str) -> Result<(), RemoteError> {
    if value.is_empty() || !value.is_ascii() || value.chars().any(char::is_whitespace) {
        return Err(RemoteError::InvalidEndpoint(format!(
            "{label} is empty or unsafe"
        )));
    }
    Ok(())
}

fn validate_endpoint_token(value: &str, label: &str, limit: usize) -> Result<(), RemoteError> {
    validate_token(value, label)?;
    if value.len() > limit {
        return Err(RemoteError::InvalidEndpoint(format!(
            "{label} exceeds its byte limit"
        )));
    }
    Ok(())
}

fn invalid_key(error: RemoteError) -> RemoteError {
    RemoteError::InvalidHostKey(error.to_string())
}
