use crate::{AuditRecord, constants::*};
use serde::Serialize;
use std::path::Path;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_peer_protocol::{Purpose, Signed};

/// One audit line with its signer and signature status. Local reads verify the
/// embedded signature; trusted merges additionally require an expected signer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEntry {
    pub record: AuditRecord,
    pub signer: Option<PeerIdentity>,
    pub signature_valid: bool,
}

impl AuditEntry {
    pub fn authenticated(&self) -> bool {
        self.signature_valid
            && self
                .signer
                .is_some_and(|signer| signer.hub() == self.record.destination)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSignature {
    Valid,
    Invalid,
    Unsigned,
}

impl AuditSignature {
    pub fn label(self) -> &'static str {
        match self {
            Self::Valid => SIGNATURE_VALID,
            Self::Invalid => SIGNATURE_INVALID,
            Self::Unsigned => SIGNATURE_UNSIGNED,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AuditSignerDocument {
    pub hub_id: yolo_hub_identity::HubId,
    pub public_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AuditEntryDocument {
    #[serde(flatten)]
    pub record: AuditRecord,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer: Option<AuditSignerDocument>,
    pub signature: AuditSignature,
}

impl From<&AuditEntry> for AuditEntryDocument {
    fn from(entry: &AuditEntry) -> Self {
        Self {
            record: entry.record.clone(),
            signer: entry.signer.map(|identity| signer_document(&identity)),
            signature: match (entry.signer, entry.signature_valid) {
                (Some(_), true) => AuditSignature::Valid,
                (Some(_), false) => AuditSignature::Invalid,
                (None, _) => AuditSignature::Unsigned,
            },
        }
    }
}

pub fn signer_document(identity: &PeerIdentity) -> AuditSignerDocument {
    AuditSignerDocument {
        hub_id: identity.hub(),
        public_key: crate::public_key_hex(identity),
    }
}

pub(crate) struct Stored {
    pub entry: AuditEntry,
    envelope: Option<Signed<AuditRecord>>,
}

pub fn read(path: &Path) -> Result<Vec<AuditEntry>, std::io::Error> {
    Ok(read_stored(path)?
        .into_iter()
        .map(|stored| stored.entry)
        .collect())
}

pub(crate) fn read_stored(path: &Path) -> Result<Vec<Stored>, std::io::Error> {
    let Ok(bytes) = std::fs::read(path) else {
        return Ok(Vec::new());
    };
    let text = String::from_utf8_lossy(&bytes);
    let mut entries = Vec::new();
    for line in text.lines() {
        if let Ok(signed) = serde_json::from_str::<Signed<AuditRecord>>(line) {
            let signature_valid = signed.verify(Purpose::Audit, signed.signer).is_ok();
            entries.push(Stored {
                entry: AuditEntry {
                    record: signed.payload.clone(),
                    signer: Some(signed.signer),
                    signature_valid,
                },
                envelope: Some(signed),
            });
        } else if let Ok(record) = serde_json::from_str::<AuditRecord>(line) {
            entries.push(Stored {
                entry: AuditEntry {
                    record,
                    signer: None,
                    signature_valid: false,
                },
                envelope: None,
            });
        }
    }
    Ok(entries)
}

pub(crate) fn from_signed(envelope: Signed<AuditRecord>) -> Stored {
    Stored {
        entry: AuditEntry {
            record: envelope.payload.clone(),
            signer: Some(envelope.signer),
            signature_valid: true,
        },
        envelope: Some(envelope),
    }
}

pub(crate) fn encode(stored: &Stored) -> Result<Vec<u8>, std::io::Error> {
    let mut body = match &stored.envelope {
        Some(envelope) => serde_json::to_vec(envelope),
        None => serde_json::to_vec(&stored.entry.record),
    }
    .map_err(std::io::Error::other)?;
    body.push(b'\n');
    Ok(body)
}
