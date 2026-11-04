use super::super::RemoteError;
use super::super::constants::*;
use super::artifact_format::binary_target;
use std::sync::Arc;
use yolo_crypto::sha256_hex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteArchitecture {
    Aarch64,
    X86_64,
}

impl RemoteArchitecture {
    pub(crate) fn path_name(self) -> &'static str {
        match self {
            Self::Aarch64 => ARCHITECTURE_AARCH64,
            Self::X86_64 => ARCHITECTURE_X86_64,
        }
    }

    pub(crate) fn matches_remote(self, value: &str) -> bool {
        match self {
            Self::Aarch64 => matches!(value, ARCHITECTURE_AARCH64 | ARCHITECTURE_ARM64),
            Self::X86_64 => matches!(value, ARCHITECTURE_X86_64 | ARCHITECTURE_AMD64),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemotePlatform {
    Linux,
    MacOs,
}

impl RemotePlatform {
    pub(crate) fn path_name(self) -> &'static str {
        match self {
            Self::Linux => PLATFORM_LINUX_PATH,
            Self::MacOs => PLATFORM_MACOS_PATH,
        }
    }

    pub(crate) fn matches_remote(self, value: &str) -> bool {
        matches!(
            (self, value),
            (Self::Linux, PLATFORM_LINUX) | (Self::MacOs, PLATFORM_DARWIN)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BinaryTarget {
    pub(super) platform: RemotePlatform,
    pub(super) architecture: RemoteArchitecture,
}

#[derive(Debug, Clone)]
pub struct RuntimeArtifact {
    version: String,
    platform: RemotePlatform,
    architecture: RemoteArchitecture,
    bytes: Arc<[u8]>,
    sha256: String,
}

impl RuntimeArtifact {
    pub fn new(
        version: impl Into<String>,
        architecture: RemoteArchitecture,
        bytes: impl Into<Arc<[u8]>>,
        expected_sha256: impl Into<String>,
    ) -> Result<Self, RemoteError> {
        let version = version.into();
        let bytes = bytes.into();
        let sha256 = expected_sha256.into();
        let target = validate(&version, &bytes, &sha256)?;
        if target.architecture != architecture {
            return Err(RemoteError::InvalidArtifact(
                "binary header does not match the declared architecture".into(),
            ));
        }
        Ok(Self {
            version,
            platform: target.platform,
            architecture,
            bytes,
            sha256,
        })
    }

    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn architecture(&self) -> RemoteArchitecture {
        self.architecture
    }
    pub fn platform(&self) -> RemotePlatform {
        self.platform
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub(crate) fn shared_bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.bytes)
    }
    pub(crate) fn validate_integrity(&self) -> Result<(), RemoteError> {
        validate(&self.version, &self.bytes, &self.sha256).map(|_| ())
    }
}

fn validate(version: &str, bytes: &[u8], sha256: &str) -> Result<BinaryTarget, RemoteError> {
    validate_version(version)?;
    if !valid_binary_size(bytes.len()) {
        return Err(RemoteError::InvalidArtifact(
            "binary size is invalid".into(),
        ));
    }
    if !valid_digest(sha256) || sha256_hex(bytes) != sha256 {
        return Err(RemoteError::RuntimeIntegrity(
            "artifact SHA-256 does not match its manifest".into(),
        ));
    }
    binary_target(bytes)
        .ok_or_else(|| RemoteError::InvalidArtifact("binary format is unsupported".into()))
}

pub(super) fn valid_binary_size(length: usize) -> bool {
    length > 0 && length <= MAX_RUNTIME_BINARY_BYTES
}

fn validate_version(version: &str) -> Result<(), RemoteError> {
    let valid = !version.is_empty()
        && version.len() <= VERSION_MAX_BYTES
        && version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'));
    if valid {
        Ok(())
    } else {
        Err(RemoteError::InvalidArtifact("version is invalid".into()))
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
