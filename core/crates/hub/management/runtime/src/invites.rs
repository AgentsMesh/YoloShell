use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use uuid::Uuid;
use yolo_paths::directory::PrivateDirectory;
use yolo_paths::{atomic_write_json, read_json};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    schema: u16,
    consumed: BTreeMap<Uuid, u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InviteLedgerError {
    Consumed,
    Full,
    Unavailable,
}

pub(crate) struct InviteLedger {
    _directory: PrivateDirectory,
    path: PathBuf,
    ledger: Ledger,
}

impl InviteLedger {
    pub(crate) fn open(root: &PrivateDirectory) -> Result<Self> {
        let directory =
            root.ensure_descendant(&root.path().join(PAIRING_DIRECTORY))?;
        let path = directory.path().join(PAIRING_LEDGER_FILE);
        let ledger = if std::fs::symlink_metadata(&path)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        {
            Ledger {
                schema: PAIRING_LEDGER_SCHEMA,
                consumed: BTreeMap::new(),
            }
        } else {
            read_json(&path)?
        };
        ensure!(ledger.schema == PAIRING_LEDGER_SCHEMA, INVALID_PAIRING_LEDGER);
        Ok(Self {
            _directory: directory,
            path,
            ledger,
        })
    }

    /// Durably records one-time use before the configuration change is applied.
    /// A crash can burn an invite; it cannot authorize a replay.
    pub(crate) fn consume(
        &mut self,
        invite_id: Uuid,
        expires_at_ms: u64,
        now_ms: u64,
    ) -> Result<(), InviteLedgerError> {
        self.prune(now_ms);
        if self.ledger.consumed.contains_key(&invite_id) {
            return Err(InviteLedgerError::Consumed);
        }
        if self.ledger.consumed.len() >= MAX_CONSUMED_INVITES {
            return Err(InviteLedgerError::Full);
        }
        self.ledger.consumed.insert(invite_id, expires_at_ms);
        atomic_write_json(&self.path, &self.ledger)
            .map_err(|_| InviteLedgerError::Unavailable)
    }

    fn prune(&mut self, now_ms: u64) {
        self.ledger
            .consumed
            .retain(|_, expires| expires.saturating_add(CLOCK_SKEW_MS) > now_ms);
    }
}

/// Invite clocks tolerate the same skew as signature verification.
const CLOCK_SKEW_MS: u64 = 60 * 1000;
