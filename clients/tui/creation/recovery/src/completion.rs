use crate::{BlockedReason, CreationRecovery, rejection, state::Recovery};
use std::time::Instant;
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::CreationQueryOutcome;

impl CreationRecovery {
    pub fn finished(
        &mut self,
        request: &ControlRequest,
        result: Result<Reply, ControlError>,
        now: Instant,
    ) -> bool {
        let Some(entry) = self.current(request, now) else {
            return false;
        };
        entry.retry(now);
        match result {
            Ok(Reply::CreationQuery { query }) => {
                if query
                    .validate_for(
                        entry.intent.request.fields().creation_token,
                        entry.intent.origin,
                    )
                    .is_err()
                {
                    entry.blocked = Some(BlockedReason::InvalidResponse);
                } else {
                    match query.outcome() {
                        CreationQueryOutcome::Missing {} | CreationQueryOutcome::Pending {} => {}
                        CreationQueryOutcome::Unknown {} => {
                            entry.blocked = Some(BlockedReason::Unknown)
                        }
                        CreationQueryOutcome::Ready { .. }
                        | CreationQueryOutcome::Finalized { .. }
                        | CreationQueryOutcome::Survivor { .. }
                        | CreationQueryOutcome::Archived { .. } => {
                            entry.resolution = Some(query);
                        }
                    }
                }
            }
            Ok(Reply::Error { code }) => entry.blocked = rejection::remote(code),
            Ok(_) => entry.blocked = Some(BlockedReason::InvalidResponse),
            Err(error) => entry.blocked = rejection::error(&error),
        }
        true
    }

    pub(crate) fn current(
        &mut self,
        request: &ControlRequest,
        now: Instant,
    ) -> Option<&mut Recovery> {
        if request.generation != self.generation {
            return None;
        }
        let Command::QueryCreation { request: query } = &request.command else {
            return None;
        };
        let entry = self
            .entries
            .get_mut(&query.creation().fields().creation_token)?;
        if entry.resolution.is_some()
            || entry.blocked.is_some()
            || entry.intent.origin != request.destination
            || entry.intent.request != *query.creation()
            || !entry
                .attempt
                .as_ref()
                .is_some_and(|attempt| attempt.matches(request, now))
        {
            return None;
        }
        Some(entry)
    }
}
