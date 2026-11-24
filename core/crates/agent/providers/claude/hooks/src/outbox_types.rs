use super::outbox_error;
use super::session_start_types::PreparedSessionStart;
use crate::claude_hooks::{
    CLAUDE_HOOK_SCHEMA_VERSION, ClaudeHookCredentials, ClaudeHookOperation, ClaudeHookRequest,
    ClaudeLifecycleEvent, INITIAL_CLAUDE_ACKNOWLEDGED_SEQUENCE, INITIAL_CLAUDE_EVENT_SEQUENCE,
    MAX_CLAUDE_OUTBOX_EVENTS,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OutboxState {
    pub schema_version: u32,
    pub invocations: VecDeque<InvocationQueue>,
    pub failed_handshakes: VecDeque<FailedHandshakeAudit>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FailedHandshakeAudit {
    #[serde(with = "crate::serde_uuid")]
    pub invocation_id: uuid::Uuid,
    pub agent: ProcessClaim,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub event_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InvocationQueue {
    pub credentials: ClaudeHookCredentials,
    pub agent: ProcessClaim,
    pub acknowledged_sequence: u64,
    pub next_sequence: u64,
    pub pending: VecDeque<QueuedRequest>,
    pub claim: Option<ProcessClaim>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub authenticated_session_start: Option<AuthenticatedSessionStart>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub prepared_session_start: Option<PreparedSessionStart>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuthenticatedSessionStart {
    pub event_sequence: u64,
    #[serde(with = "crate::serde_uuid")]
    pub event_id: uuid::Uuid,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct QueuedRequest {
    pub request: ClaudeHookRequest,
    pub transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProcessClaim {
    pub(super) boot: crate::process::boot_identity::BootIdentity,
    pub(super) pid: u32,
    pub(super) start_time: u64,
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/outbox_current_shape_tests.rs"]
mod current_shape_tests;

impl ProcessClaim {
    pub(super) fn current() -> Result<Self> {
        Self::for_pid(std::process::id())
    }

    fn parent() -> Result<Self> {
        let pid = unsafe { libc::getppid() };
        ensure!(pid > 0, outbox_error::PARENT_PROCESS_MISSING);
        Self::for_pid(pid as u32)
    }

    fn for_pid(pid: u32) -> Result<Self> {
        Ok(Self {
            boot: crate::process::boot_identity::BootIdentity::current(),
            pid,
            start_time: crate::process::process_start_time(pid)?,
        })
    }

    pub(super) fn relation(&self) -> crate::process::ProcessIdentityRelation {
        match self.boot.relation_to_current() {
            crate::process::BootIdentityRelation::Different => {
                crate::process::ProcessIdentityRelation::Gone
            }
            crate::process::BootIdentityRelation::Same => {
                crate::process::process_identity_relation(self.pid, self.start_time)
            }
            crate::process::BootIdentityRelation::Unknown => {
                match crate::process::process_identity_relation(self.pid, self.start_time) {
                    crate::process::ProcessIdentityRelation::Gone => {
                        crate::process::ProcessIdentityRelation::Gone
                    }
                    crate::process::ProcessIdentityRelation::Matching
                    | crate::process::ProcessIdentityRelation::Unknown => {
                        crate::process::ProcessIdentityRelation::Unknown
                    }
                }
            }
        }
    }

    pub(super) fn identity(&self) -> crate::process::ProcessIdentity {
        crate::process::ProcessIdentity {
            pid: self.pid,
            start_time: self.start_time,
        }
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) fn from_identity(identity: crate::process::ProcessIdentity) -> Self {
        Self {
            boot: crate::process::boot_identity::BootIdentity::current(),
            pid: identity.pid,
            start_time: identity.start_time,
        }
    }

    #[cfg(test)]
    pub(super) fn from_identity_on_boot(
        identity: crate::process::ProcessIdentity,
        boot_id: &str,
    ) -> Self {
        Self {
            boot: crate::process::boot_identity::BootIdentity::from_value(boot_id),
            pid: identity.pid,
            start_time: identity.start_time,
        }
    }
}

impl InvocationQueue {
    pub(super) fn new(credentials: ClaudeHookCredentials) -> Result<Self> {
        Ok(Self {
            credentials,
            agent: ProcessClaim::parent()?,
            acknowledged_sequence: INITIAL_CLAUDE_ACKNOWLEDGED_SEQUENCE,
            next_sequence: INITIAL_CLAUDE_EVENT_SEQUENCE,
            pending: VecDeque::new(),
            claim: None,
            authenticated_session_start: None,
            prepared_session_start: None,
        })
    }

    pub(super) fn enqueue(
        &mut self,
        event: ClaudeLifecycleEvent,
        transcript_identity: Option<crate::paths::identity::ObjectIdentity>,
    ) -> Result<u64> {
        ensure!(
            self.pending.len() < MAX_CLAUDE_OUTBOX_EVENTS,
            outbox_error::EVENT_CAPACITY
        );
        let sequence = self.next_sequence;
        self.next_sequence = sequence
            .checked_add(1)
            .context(outbox_error::EVENT_SEQUENCE_OVERFLOW)?;
        self.pending.push_back(QueuedRequest {
            request: ClaudeHookRequest {
                schema_version: CLAUDE_HOOK_SCHEMA_VERSION,
                credentials: self.credentials,
                operation: ClaudeHookOperation::Lifecycle {
                    event_sequence: sequence,
                    event,
                },
            },
            transcript_identity,
        });
        Ok(sequence)
    }

    pub(super) fn recoverable(&self) -> bool {
        self.authenticated_session_start.is_some()
            && !self.pending.is_empty()
            && self.agent.relation() == crate::process::ProcessIdentityRelation::Gone
            && self.claim.is_none_or(|claim| {
                claim.relation() == crate::process::ProcessIdentityRelation::Gone
            })
    }
}
