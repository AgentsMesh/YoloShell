use crate::{
    AuditOutcome, AuditRecord, MAX_AUDIT_RECORDS,
    constants::*,
    entry::{Stored, encode, from_signed, read_stored},
};
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use yolo_hub_delegation::Signer;
use yolo_hub_identity::HubId;
use yolo_hub_peer_protocol::{Purpose, Signed};
use yolo_hub_rpc_protocol::Command;

pub struct AuditLog {
    path: PathBuf,
    signer: Arc<dyn Signer + Send + Sync>,
    state: Mutex<State>,
}

struct State {
    sequence: u64,
    records: VecDeque<Stored>,
}

impl AuditLog {
    pub fn open(
        path: PathBuf,
        signer: Arc<dyn Signer + Send + Sync>,
    ) -> Result<Self, std::io::Error> {
        let mut records = read_stored(&path)?.into_iter().collect::<VecDeque<_>>();
        let sequence = records
            .back()
            .map_or(0, |stored| stored.entry.record.sequence);
        while records.len() > MAX_AUDIT_RECORDS {
            records.pop_front();
        }
        Ok(Self {
            path,
            signer,
            state: Mutex::new(State { sequence, records }),
        })
    }

    pub fn record(
        &self,
        principal: &str,
        destination: HubId,
        command: &Command,
        outcome: AuditOutcome,
    ) -> Result<(), std::io::Error> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.sequence = state.sequence.saturating_add(1);
        let record = AuditRecord {
            sequence: state.sequence,
            at_ms: now_ms(),
            principal: principal.into(),
            method: method_name(command),
            destination,
            target: crate::extract::target(command),
            request_id: crate::extract::request_id(command),
            outcome,
        };
        let signed = Signed::new(record, Purpose::Audit, &*self.signer)
            .map_err(|_| std::io::Error::other(AUDIT_SIGNING_FAILED))?;
        let stored = from_signed(signed);
        let body = encode(&stored)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(&self.path)?;
        file.write_all(&body)?;
        state.records.push_back(stored);
        if state.records.len() > MAX_AUDIT_RECORDS {
            state.records.pop_front();
            rewrite(&self.path, &state.records)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn records(&self) -> Vec<AuditRecord> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .records
            .iter()
            .map(|stored| stored.entry.record.clone())
            .collect()
    }
}

fn rewrite(path: &Path, records: &VecDeque<Stored>) -> Result<(), std::io::Error> {
    let temporary = path.with_extension("jsonl.tmp");
    let mut body = Vec::new();
    for record in records {
        body.extend(encode(record)?);
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)?;
    file.write_all(&body)?;
    file.sync_all()?;
    std::fs::rename(temporary, path)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn method_name(command: &Command) -> String {
    serde_json::to_value(command.method())
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".into())
}
