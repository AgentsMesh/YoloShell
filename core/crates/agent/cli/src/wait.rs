use yolo_agent_baseline::{Baseline, current, latest_request};
use crate::constants::*;
use crate::observe;
use crate::wait_pages::{absorb, is_assistant_text, rebase};
use anyhow::Result;
use std::time::Duration;
use tokio::time::Instant;
use yolo_conversation_protocol::{ConversationEntry, ConversationError, ConversationReadResult};
use yolo_hub_agent_protocol::{Activity, AgentSnapshot, Lifecycle};
use yolo_hub_client::LocalClient;
use yolo_hub_conversation_client::{self as conversation, ConversationClientError};
use yolo_hub_identity::{AgentTarget, RuntimeTarget};

pub enum WaitOutcome {
    Completed,
    Approval,
    Exited,
    ExecutionEnded,
    TimedOut,
}

pub struct WaitResult {
    pub outcome: WaitOutcome,
    pub snapshot: Option<AgentSnapshot>,
    pub entries: Vec<ConversationEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Progress {
    Completed,
    AwaitingReply,
    Running,
}

/// The turn settles when the agent leaves its working state. A provider may
/// report the settle slightly before its transcript flush, so a readable
/// conversation gets a bounded grace period to deliver the assistant entry.
pub(crate) fn progress(
    activity: Activity,
    seen_working: bool,
    has_assistant: bool,
    grace_elapsed: bool,
) -> Progress {
    let settled = !matches!(activity, Activity::Working | Activity::WaitingForApproval);
    if settled && has_assistant {
        return Progress::Completed;
    }
    if settled && seen_working {
        return if grace_elapsed {
            Progress::Completed
        } else {
            Progress::AwaitingReply
        };
    }
    Progress::Running
}

pub async fn complete(
    client: &mut LocalClient,
    target: AgentTarget,
    runtime: RuntimeTarget,
    mut baseline: Baseline,
    timeout: Duration,
) -> Result<WaitResult> {
    let deadline = Instant::now() + timeout;
    let mut snapshot: Option<AgentSnapshot> = None;
    let mut new_entries: Vec<ConversationEntry> = Vec::new();
    let mut seen_working = false;
    let mut settled_at: Option<Instant> = None;
    loop {
        if Instant::now() >= deadline {
            return Ok(result(WaitOutcome::TimedOut, snapshot, new_entries));
        }
        let Some(current) = observe::optional(client, target).await? else {
            return Ok(result(WaitOutcome::ExecutionEnded, snapshot, new_entries));
        };
        seen_working |= current.fields().activity == Activity::Working;
        let watched = watch_once(client, target, runtime, &mut baseline, &mut new_entries).await?;
        let fields = current.fields();
        let settled = !matches!(
            fields.activity,
            Activity::Working | Activity::WaitingForApproval
        );
        if fields.activity == Activity::WaitingForApproval {
            return Ok(result(WaitOutcome::Approval, Some(current), new_entries));
        }
        let grace_elapsed = settled_at.is_some_and(|at| at.elapsed() >= REPLY_GRACE);
        match progress(
            fields.activity,
            seen_working,
            new_entries.iter().any(is_assistant_text),
            grace_elapsed,
        ) {
            Progress::Completed => {
                return Ok(result(WaitOutcome::Completed, Some(current), new_entries));
            }
            Progress::AwaitingReply => {
                settled_at.get_or_insert_with(Instant::now);
            }
            Progress::Running => {
                if !settled {
                    settled_at = None;
                }
            }
        }
        if matches!(fields.lifecycle, Lifecycle::Exited { .. }) {
            return Ok(result(WaitOutcome::Exited, Some(current), new_entries));
        }
        snapshot = Some(current);
        if !watched {
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }
}

async fn watch_once(
    client: &mut LocalClient,
    target: AgentTarget,
    runtime: RuntimeTarget,
    baseline: &mut Baseline,
    new_entries: &mut Vec<ConversationEntry>,
) -> Result<bool> {
    let Some(descriptor) = baseline.descriptor.clone() else {
        return Ok(false);
    };
    let request = latest_request(&descriptor, baseline.revision, baseline.generation);
    let deadline = Instant::now() + Duration::from_millis(u64::from(WATCH_WAIT_MS)) + RPC_TIMEOUT;
    match conversation::watch(client, request, WATCH_WAIT_MS, deadline).await {
        Ok(ConversationReadResult::Page { page }) => {
            absorb(baseline, new_entries, page);
            Ok(true)
        }
        Ok(ConversationReadResult::Unchanged { .. }) => Ok(true),
        Err(ConversationClientError::Conversation(ConversationError::HistoryChanged)) => {
            if let Some((descriptor, page)) = current(client, target, runtime).await? {
                rebase(baseline, new_entries, descriptor, page);
            }
            Ok(false)
        }
        // A provider can rewrite its transcript while the turn ends. Keep the
        // reader and retry; the settled-turn grace bounds the wait.
        Err(ConversationClientError::Conversation(_)) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn result(
    outcome: WaitOutcome,
    snapshot: Option<AgentSnapshot>,
    entries: Vec<ConversationEntry>,
) -> WaitResult {
    WaitResult {
        outcome,
        snapshot,
        entries,
    }
}
