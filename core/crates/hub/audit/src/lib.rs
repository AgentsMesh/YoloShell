//! Durable authority audit for platform mutations. The log carries the local
//! principal, method, target, outcome and idempotency token; every new line is
//! signed by the Hub identity so exported logs can be authenticated offline.
//! It is evidence, never authorization.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Command;

mod constants;
mod dispatch;
mod entry;
mod extract;
mod log;
mod trust;

pub use dispatch::AuditedDispatcher;
pub use entry::{
    AuditEntry, AuditEntryDocument, AuditSignature, AuditSignerDocument, read, signer_document,
};
pub use log::AuditLog;
pub use trust::{AuditTrust, AuditTrustError, public_key_hex};

pub const AUDIT_FILE: &str = "audit.jsonl";
pub const MAX_AUDIT_RECORDS: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Accepted,
    Denied,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditRecord {
    pub sequence: u64,
    pub at_ms: i64,
    pub principal: String,
    pub method: String,
    pub destination: HubId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<Uuid>,
    pub outcome: AuditOutcome,
}

pub fn read_export(
    path: &std::path::Path,
    after: Option<u64>,
    limit: usize,
) -> Result<Vec<serde_json::Value>, std::io::Error> {
    let Ok(bytes) = std::fs::read(path) else {
        return Ok(Vec::new());
    };
    let text = String::from_utf8_lossy(&bytes);
    let mut lines = Vec::new();
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let sequence = value
            .get("payload")
            .and_then(|payload| payload.get("sequence"))
            .or_else(|| value.get("sequence"))
            .and_then(serde_json::Value::as_u64);
        if sequence.is_some_and(|sequence| after.is_none_or(|after| sequence > after)) {
            lines.push(value);
            if lines.len() >= limit {
                break;
            }
        }
    }
    Ok(lines)
}

pub fn should_audit(command: &Command) -> bool {
    use Command::*;
    if let RemoteSetup { call } = command {
        return call.mutates();
    }
    matches!(
        command,
        Create { .. }
            | Finalize { .. }
            | TerminalRun { .. }
            | Terminal { .. }
            | CleanupTerminate { .. }
            | CleanupArchive { .. }
            | AgentStart { .. }
            | AgentStop { .. }
            | AgentPrompt { .. }
            | AgentControl { .. }
            | UpdateAgentRun { .. }
            | CreateWorkspace { .. }
            | UpdateWorkspace { .. }
            | CreateWorktree { .. }
            | RemoveWorktree { .. }
            | Configure { .. }
            | ConfigureSshProfiles { .. }
            | ConnectSshProfile { .. }
            | RemoteSetup { .. }
            | ConfigureClientAccess { .. }
    )
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
