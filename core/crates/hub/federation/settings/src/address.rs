use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
pub use yolo_ssh_profile_protocol::SshProfileRef;
use yolo_ssh_profile_protocol::validate_profile_id;

/// One outbound peer transport plus the provider Hub's absolute Unix socket.
///
/// `Ssh` is the user-facing peer link: the local OpenSSH client supplies
/// connectivity, host trust and credentials. `Profile` remains the managed
/// remote-setup binding owned by the Hub broker; it is not created by
/// `yolo machine add`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PeerAddress {
    Ssh(SshAddress),
    Profile(ProfileAddress),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SshAddressWire", into = "SshAddressWire")]
pub struct SshAddress {
    pub ssh: String,
    pub socket: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SshAddressWire {
    ssh: String,
    socket: PathBuf,
}

impl TryFrom<SshAddressWire> for SshAddress {
    type Error = anyhow::Error;
    fn try_from(value: SshAddressWire) -> Result<Self> {
        let address = Self {
            ssh: value.ssh,
            socket: value.socket,
        };
        validate_ssh_target(&address.ssh)?;
        validate_socket(&address.socket)?;
        Ok(address)
    }
}

impl From<SshAddress> for SshAddressWire {
    fn from(value: SshAddress) -> Self {
        Self {
            ssh: value.ssh,
            socket: value.socket,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ProfileAddressWire", into = "ProfileAddressWire")]
pub struct ProfileAddress {
    pub profile: String,
    pub socket: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileAddressWire {
    profile: String,
    socket: PathBuf,
}

impl TryFrom<ProfileAddressWire> for ProfileAddress {
    type Error = anyhow::Error;
    fn try_from(value: ProfileAddressWire) -> Result<Self> {
        let address = Self {
            profile: value.profile,
            socket: value.socket,
        };
        validate_profile_id(&address.profile)?;
        validate_socket(&address.socket)?;
        Ok(address)
    }
}

impl From<ProfileAddress> for ProfileAddressWire {
    fn from(value: ProfileAddress) -> Self {
        Self {
            profile: value.profile,
            socket: value.socket,
        }
    }
}

impl PeerAddress {
    pub fn ssh(target: impl Into<String>, socket: PathBuf) -> Result<Self> {
        let value = Self::Ssh(SshAddress {
            ssh: target.into(),
            socket,
        });
        value.validate()?;
        Ok(value)
    }
    pub fn profile(profile: impl Into<String>, socket: PathBuf) -> Result<Self> {
        let value = Self::Profile(ProfileAddress {
            profile: profile.into(),
            socket,
        });
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Ssh(address) => {
                validate_ssh_target(&address.ssh)?;
                validate_socket(&address.socket)
            }
            Self::Profile(address) => {
                validate_profile_id(&address.profile)?;
                validate_socket(&address.socket)
            }
        }
    }
    pub fn socket(&self) -> &Path {
        match self {
            Self::Ssh(address) => &address.socket,
            Self::Profile(address) => &address.socket,
        }
    }
    pub fn ssh_target(&self) -> Option<&str> {
        match self {
            Self::Ssh(address) => Some(&address.ssh),
            Self::Profile(_) => None,
        }
    }
    pub fn profile_id(&self) -> Option<&str> {
        match self {
            Self::Ssh(_) => None,
            Self::Profile(address) => Some(&address.profile),
        }
    }
}

/// An SSH destination is passed as one argv element, so it must not be an
/// option, an empty string or something the remote shell could split.
pub fn validate_ssh_target(target: &str) -> Result<()> {
    ensure!(
        !target.is_empty()
            && target.len() <= MAX_SSH_TARGET_BYTES
            && !target.starts_with('-'),
        INVALID
    );
    ensure!(
        target.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'.' | b'-' | b'_' | b'@' | b'%' | b'+' | b':' | b'[' | b']'
                )
        }),
        INVALID
    );
    Ok(())
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
