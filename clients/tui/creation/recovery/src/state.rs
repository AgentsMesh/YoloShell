use crate::{BlockedReason, attempt::Attempt, constants::*};
use anyhow::{Result, ensure};
use std::{collections::BTreeMap, time::Instant};
use uuid::Uuid;
use yolo_hub_terminal_protocol::{CreateShellRequest, CreationQueryResult};
use yolo_tui_creation_journal::{CreationIntent, MAX_PENDING_CREATIONS};

#[derive(Default)]
pub struct CreationRecovery {
    pub(crate) entries: BTreeMap<Uuid, Recovery>,
    pub(crate) generation: Option<Uuid>,
}

pub(crate) struct Recovery {
    pub intent: CreationIntent,
    pub attempt: Option<Attempt>,
    pub retry_at: Instant,
    pub blocked: Option<BlockedReason>,
    pub resolution: Option<CreationQueryResult>,
}

impl CreationRecovery {
    /// Exact repetition preserves attempts and resolutions. Neither return value
    /// permits Create delivery; this owner only queries already durable intents.
    pub fn retain(&mut self, intent: CreationIntent, now: Instant) -> Result<bool> {
        CreationIntent::new(intent.origin, intent.request.clone())?;
        if let Some(existing) = self
            .entries
            .values()
            .find(|entry| overlaps(&entry.intent.request, &intent.request))
        {
            ensure!(existing.intent == intent, CONFLICT);
            return Ok(false);
        }
        ensure!(self.entries.len() < MAX_PENDING_CREATIONS, CAPACITY);
        self.entries.insert(
            intent.request.fields().creation_token,
            Recovery {
                intent,
                attempt: None,
                retry_at: now,
                blocked: None,
                resolution: None,
            },
        );
        Ok(true)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Request IDs and tokens cannot be reused, including at another origin.
    pub fn contains_request(&self, request: &CreateShellRequest) -> bool {
        self.entries
            .values()
            .any(|entry| overlaps(&entry.intent.request, request))
    }

    /// Kept until the caller durably promotes or settles the original journal
    /// intent. Reading this list does not consume confirmation evidence.
    pub fn resolutions(&self) -> Vec<(CreationIntent, CreationQueryResult)> {
        self.entries
            .values()
            .filter_map(|entry| {
                entry
                    .resolution
                    .clone()
                    .map(|result| (entry.intent.clone(), result))
            })
            .collect()
    }

    /// The caller must first complete the matching durable journal change.
    /// Removing an exact record also retires any still-queued local attempt.
    pub fn settled(&mut self, exact: &CreationIntent) -> Result<bool> {
        let Some(entry) = self
            .entries
            .values()
            .find(|entry| overlaps(&entry.intent.request, &exact.request))
        else {
            return Ok(false);
        };
        ensure!(entry.intent == *exact, CONFLICT);
        self.entries.remove(&exact.request.fields().creation_token);
        Ok(true)
    }
}

pub(crate) fn overlaps(left: &CreateShellRequest, right: &CreateShellRequest) -> bool {
    left.fields().request_id == right.fields().request_id
        || left.fields().creation_token == right.fields().creation_token
}

impl Recovery {
    pub fn retry(&mut self, now: Instant) {
        self.attempt.take();
        self.retry_at = now + RETRY_DELAY;
    }
}
