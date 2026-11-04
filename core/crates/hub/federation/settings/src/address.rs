use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, SocketAddr},
    num::NonZeroU16,
};
pub use yolo_ssh_profile_protocol::SshProfileRef;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(from = "TransportWire", into = "TransportWire")]
pub enum DialTransport {
    #[default]
    Tcp,
    Ssh {
        profile: SshProfileRef,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum TransportWire {
    Tcp {},
    Ssh { profile: SshProfileRef },
}
impl From<TransportWire> for DialTransport {
    fn from(value: TransportWire) -> Self {
        match value {
            TransportWire::Tcp {} => Self::Tcp,
            TransportWire::Ssh { profile } => Self::Ssh { profile },
        }
    }
}
impl From<DialTransport> for TransportWire {
    fn from(value: DialTransport) -> Self {
        match value {
            DialTransport::Tcp => Self::Tcp {},
            DialTransport::Ssh { profile } => Self::Ssh { profile },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerAddress {
    pub host: String,
    pub port: NonZeroU16,
    pub transport: DialTransport,
}

impl PeerAddress {
    pub fn tcp(host: impl Into<String>, port: NonZeroU16) -> Self {
        Self {
            host: host.into(),
            port,
            transport: DialTransport::Tcp,
        }
    }
    pub fn ssh(profile: SshProfileRef, endpoint: SocketAddr) -> Result<Self> {
        ensure!(
            !matches!(endpoint, SocketAddr::V6(address) if address.scope_id() != 0 || address.flowinfo() != 0),
            INVALID
        );
        let value = Self {
            host: endpoint.ip().to_string(),
            port: NonZeroU16::new(endpoint.port()).ok_or_else(|| anyhow::anyhow!(INVALID))?,
            transport: DialTransport::Ssh { profile },
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.host.is_empty()
                && self.host.len() <= MAX_HOST_BYTES
                && !self
                    .host
                    .chars()
                    .any(|character| character.is_whitespace() || character.is_control()),
            INVALID
        );
        if matches!(self.transport, DialTransport::Ssh { .. }) {
            let address: IpAddr = self.host.parse()?;
            ensure!(
                address.is_loopback() && address.to_string() == self.host,
                INVALID
            );
        }
        Ok(())
    }
    pub fn ssh_endpoint(&self) -> Result<SocketAddr> {
        self.validate()?;
        ensure!(matches!(self.transport, DialTransport::Ssh { .. }), INVALID);
        Ok(SocketAddr::new(self.host.parse()?, self.port.get()))
    }
}
