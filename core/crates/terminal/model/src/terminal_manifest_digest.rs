use super::TerminalManifest;
use super::constants::{
    TERMINAL_MANIFEST_DIGEST_DOMAIN, TERMINAL_MANIFEST_DIGEST_INVALID_ENCODING_DETAIL,
    TERMINAL_MANIFEST_NON_UNICODE_WORKING_DIRECTORY_DETAIL, recovery_state_name,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};
use std::{fmt, str::FromStr};

const SHA256_BYTES: usize = 32;
const SHA256_HEX_CHARACTERS: usize = SHA256_BYTES * 2;
const HEX_ALPHABET: &[u8; 16] = b"0123456789abcdef";
const HIGH_NIBBLE_SHIFT: u8 = 4;
const LOW_NIBBLE_MASK: u8 = 0x0f;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalManifestDigest(String);

impl TerminalManifestDigest {
    pub fn capture(manifest: &TerminalManifest) -> Result<Self, TerminalManifestDigestError> {
        let TerminalManifest {
            schema_version,
            terminal_id,
            name,
            command,
            args,
            cwd,
            cols,
            rows,
            created_at_ms,
            updated_at_ms,
            archived,
            recovery_state,
        } = manifest;
        let cwd = cwd
            .to_str()
            .ok_or(TerminalManifestDigestError::NonUnicodeWorkingDirectory)?;
        let mut frame = DigestFrame::new();
        frame.unsigned16(*schema_version);
        frame.bytes(terminal_id.as_bytes());
        frame.string(name);
        frame.string(command);
        frame.unsigned64(args.len() as u64);
        for argument in args {
            frame.string(argument);
        }
        frame.string(cwd);
        frame.unsigned16(*cols);
        frame.unsigned16(*rows);
        frame.signed64(*created_at_ms);
        frame.signed64(*updated_at_ms);
        frame.boolean(*archived);
        frame.string(recovery_state_name(*recovery_state));
        Ok(Self(encode(frame.finish())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TerminalManifestDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for TerminalManifestDigest {
    type Err = TerminalManifestDigestError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != SHA256_HEX_CHARACTERS
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(TerminalManifestDigestError::InvalidEncoding);
        }
        Ok(Self(value.to_owned()))
    }
}

impl Serialize for TerminalManifestDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TerminalManifestDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalManifestDigestError {
    InvalidEncoding,
    NonUnicodeWorkingDirectory,
}

impl fmt::Display for TerminalManifestDigestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let detail = match self {
            Self::InvalidEncoding => TERMINAL_MANIFEST_DIGEST_INVALID_ENCODING_DETAIL,
            Self::NonUnicodeWorkingDirectory => {
                TERMINAL_MANIFEST_NON_UNICODE_WORKING_DIRECTORY_DETAIL
            }
        };
        formatter.write_str(detail)
    }
}

impl std::error::Error for TerminalManifestDigestError {}

struct DigestFrame(Sha256);

impl DigestFrame {
    fn new() -> Self {
        let mut hasher = Sha256::new();
        hasher.update(TERMINAL_MANIFEST_DIGEST_DOMAIN);
        Self(hasher)
    }

    fn bytes(&mut self, value: &[u8]) {
        self.unsigned64(value.len() as u64);
        self.0.update(value);
    }

    fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn unsigned16(&mut self, value: u16) {
        self.0.update(value.to_be_bytes());
    }

    fn unsigned64(&mut self, value: u64) {
        self.0.update(value.to_be_bytes());
    }

    fn signed64(&mut self, value: i64) {
        self.0.update(value.to_be_bytes());
    }

    fn boolean(&mut self, value: bool) {
        self.0.update([u8::from(value)]);
    }

    fn finish(self) -> [u8; SHA256_BYTES] {
        self.0.finalize().into()
    }
}

fn encode(bytes: [u8; SHA256_BYTES]) -> String {
    let mut encoded = String::with_capacity(SHA256_HEX_CHARACTERS);
    for byte in bytes {
        encoded.push(HEX_ALPHABET[usize::from(byte >> HIGH_NIBBLE_SHIFT)] as char);
        encoded.push(HEX_ALPHABET[usize::from(byte & LOW_NIBBLE_MASK)] as char);
    }
    encoded
}
