use super::constants::MAX_IDENTITY_FILES;
use super::credential_binding;
use super::credential_binding::CREDENTIAL_BINDING_ERROR;
use super::identity_passphrase_constants::*;
use anyhow::{Context, Result, ensure};
use std::io::{IsTerminal, Read};
use std::sync::Arc;
use zeroize::Zeroizing;

#[path = "identity_passphrase_wire.rs"]
pub(super) mod wire;
use wire::{EncodedEnvelope, Envelope, EnvelopeBinding};
#[path = "identity_passphrase_codec.rs"]
mod codec;

const INPUT_CONTEXT: &str = "read SSH identity passphrases from private stdin";
const INTERACTIVE_INPUT_ERROR: &str =
    "refusing to read SSH identity passphrases from an interactive terminal";
const INPUT_LIMIT_ERROR: &str = "SSH identity passphrase input exceeds its limit";
const INPUT_VERSION_ERROR: &str = "unsupported SSH identity passphrase input version";
const INPUT_COUNT_ERROR: &str = "SSH identity passphrase count differs from identity count";
const PASSPHRASE_LIMIT_ERROR: &str = "SSH identity passphrase exceeds its limit";
pub(super) const CREDENTIAL_REVISION_ERROR: &str = "invalid SSH identity credential revision";
pub const IDENTITY_PASSPHRASE_WIRE_VERSION: u16 = 2;
const WIRE_VERSION: u16 = IDENTITY_PASSPHRASE_WIRE_VERSION;
pub(super) const MAX_PASSPHRASE_BYTES: usize = 4 * 1_024;
const ENVELOPE_OVERHEAD_BYTES: usize = 4_096;
const MAX_INPUT_BYTES: usize = MAX_IDENTITY_FILES * MAX_PASSPHRASE_BYTES + ENVELOPE_OVERHEAD_BYTES;
const OVERFLOW_READ_BYTES: u64 = (MAX_INPUT_BYTES + 1) as u64;

pub struct IdentityPassphrases {
    pub(super) provided: bool,
    pub(super) binding: Option<EnvelopeBinding>,
    values: Vec<Option<Arc<Zeroizing<String>>>>,
}

impl IdentityPassphrases {
    pub fn none() -> Self {
        Self {
            provided: false,
            binding: None,
            values: Vec::new(),
        }
    }

    pub fn read_stdin() -> Result<Self> {
        let stdin = std::io::stdin();
        let interactive_terminal = stdin.is_terminal();
        Self::read_checked(stdin, interactive_terminal)
    }

    pub fn read_checked(reader: impl Read, interactive_terminal: bool) -> Result<Self> {
        ensure!(!interactive_terminal, INTERACTIVE_INPUT_ERROR);
        let mut encoded = Zeroizing::new(Vec::with_capacity(OVERFLOW_READ_BYTES as usize));
        reader
            .take(OVERFLOW_READ_BYTES)
            .read_to_end(&mut encoded)
            .context(INPUT_CONTEXT)?;
        ensure!(encoded.len() <= MAX_INPUT_BYTES, INPUT_LIMIT_ERROR);
        let envelope: Envelope = codec::decode(&encoded).context(INPUT_CONTEXT)?;
        ensure!(envelope.version == WIRE_VERSION, INPUT_VERSION_ERROR);
        let binding = EnvelopeBinding {
            binding_nonce: envelope.binding_nonce,
            host_id: envelope.host_id,
            identity_credential_revision: envelope.identity_credential_revision,
            identity_paths_sha256: envelope.identity_paths_sha256,
        };
        binding.validate()?;
        ensure!(
            envelope.passphrases.len() <= MAX_IDENTITY_FILES,
            INPUT_COUNT_ERROR
        );
        ensure!(
            envelope
                .passphrases
                .iter()
                .flatten()
                .all(|value| !value.0.is_empty() && value.0.len() <= MAX_PASSPHRASE_BYTES),
            PASSPHRASE_LIMIT_ERROR
        );
        Ok(Self {
            provided: true,
            binding: Some(binding),
            values: envelope
                .passphrases
                .into_iter()
                .map(|value| value.map(|secret| Arc::new(secret.0)))
                .collect(),
        })
    }

    pub fn encode(&self) -> Result<Zeroizing<Vec<u8>>> {
        self.encode_rebound(None)
    }

    pub fn encode_rebound(&self, nonce: Option<&str>) -> Result<Zeroizing<Vec<u8>>> {
        let mut binding = self.binding.clone().context(CREDENTIAL_BINDING_ERROR)?;
        if let Some(value) = nonce {
            credential_binding::validate_nonce(value)?;
            binding.binding_nonce = value.to_owned();
        }
        let passphrases = self
            .values
            .iter()
            .map(|value| value.as_deref().map(|secret| secret.as_str()))
            .collect();
        let value = yolo_capnp_sensitive::OwnedValue(
            yolo_capnp_codec::to_value(&EncodedEnvelope {
                version: WIRE_VERSION,
                binding_nonce: &binding.binding_nonce,
                host_id: &binding.host_id,
                identity_credential_revision: binding.identity_credential_revision.as_deref(),
                identity_paths_sha256: &binding.identity_paths_sha256,
                passphrases,
            })
            .context(INPUT_CONTEXT)?,
        );
        yolo_capnp_sensitive::encode(&value.0).context(INPUT_CONTEXT)
    }

    pub(crate) fn validate_count(&self, count: usize) -> Result<()> {
        ensure!(
            !self.provided || self.values.len() == count,
            INPUT_COUNT_ERROR
        );
        Ok(())
    }

    pub(crate) fn get(&self, index: usize) -> Option<Arc<Zeroizing<String>>> {
        self.values.get(index).and_then(Clone::clone)
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/identity_passphrase_adversarial_tests.rs"]
mod adversarial_tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/identity_passphrases_tests.rs"]
mod tests;
