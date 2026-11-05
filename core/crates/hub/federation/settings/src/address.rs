use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
pub use yolo_ssh_profile_protocol::SshProfileRef;
use yolo_ssh_profile_protocol::validate_profile_id;

/// One SSH profile and the remote Hub's absolute Unix peer socket.
///
/// The outer SSH connection provides the path and host trust; the Hub identity
/// is established by the peer link handshake.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "PeerAddressWire", into = "PeerAddressWire")]
pub struct PeerAddress {
    pub profile: String,
    pub socket: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PeerAddressWire {
    profile: String,
    socket: PathBuf,
}

impl TryFrom<PeerAddressWire> for PeerAddress {
    type Error = anyhow::Error;
    fn try_from(value: PeerAddressWire) -> Result<Self> {
        Self::ssh(value.profile, value.socket)
    }
}

impl From<PeerAddress> for PeerAddressWire {
    fn from(value: PeerAddress) -> Self {
        Self {
            profile: value.profile,
            socket: value.socket,
        }
    }
}

impl PeerAddress {
    pub fn ssh(profile: impl Into<String>, socket: PathBuf) -> Result<Self> {
        let value = Self {
            profile: profile.into(),
            socket,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        validate_profile_id(&self.profile)?;
        validate_socket(&self.socket)
    }
}

/// Unix socket paths must fit the kernel `sun_path` budget and name a
/// canonical absolute location without traversal or control bytes.
pub fn validate_socket(path: &Path) -> Result<()> {
    ensure!(path.is_absolute() && path.to_str().is_some(), INVALID);
    let raw = path.to_str().ok_or_else(|| anyhow::anyhow!(INVALID))?;
    ensure!(
        !raw.split('/').any(|part| part == "." || part == ".."),
        INVALID
    );
    let encoded = path.as_os_str().as_encoded_bytes();
    ensure!(
        !encoded.is_empty() && encoded.len() <= MAX_SOCKET_PATH_BYTES,
        INVALID
    );
    ensure!(
        path.components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_))),
        INVALID
    );
    ensure!(!encoded.iter().any(|byte| byte.is_ascii_control()), INVALID);
    Ok(())
}
