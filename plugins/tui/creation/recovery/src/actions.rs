use crate::{CreationRecovery, attempt::Attempt};
use std::time::Instant;
use uuid::Uuid;
use yolo_hub_client_channels::{ControlRequest, MAX_QUERY_CHANNELS, RPC_TIMEOUT, RequestScope};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::CreationQueryRequest;

impl CreationRecovery {
    /// Call with None on disconnect to retire queued attempts immediately. A new
    /// generation creates new scopes; retained strong resolutions remain intact.
    pub fn prepare(&mut self, generation: Option<Uuid>, now: Instant) -> Vec<ControlRequest> {
        self.generation = generation.filter(|value| !value.is_nil());
        for entry in self.entries.values_mut() {
            if let Some(attempt) = &entry.attempt {
                if Some(attempt.generation) != self.generation {
                    entry.attempt.take();
                    entry.retry_at = now;
                } else if attempt.validity.deadline() <= now || !attempt.validity.is_current() {
                    entry.retry(now);
                }
            }
        }
        let Some(generation) = self.generation else {
            return Vec::new();
        };
        let in_flight = self
            .entries
            .values()
            .filter(|entry| entry.attempt.is_some())
            .count();
        let mut eligible: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(token, entry)| {
                (entry.attempt.is_none()
                    && entry.blocked.is_none()
                    && entry.resolution.is_none()
                    && entry.retry_at <= now)
                    .then_some((entry.retry_at, *token))
            })
            .collect();
        // Older untouched records precede retries, even with a slow UI tick.
        eligible.sort_unstable();
        let mut requests = Vec::new();
        for (_, token) in eligible
            .into_iter()
            .take(MAX_QUERY_CHANNELS.saturating_sub(in_flight))
        {
            let entry = self
                .entries
                .get_mut(&token)
                .expect(crate::constants::CONFLICT);
            let command = Command::QueryCreation {
                request: CreationQueryRequest::new(entry.intent.request.clone()),
            };
            let scope = RequestScope::default();
            let validity = scope.validity_until(now + command.operation_timeout(RPC_TIMEOUT));
            requests.push(ControlRequest {
                validity: Some(validity.clone()),
                generation: Some(generation),
                destination: entry.intent.origin,
                command,
            });
            entry.attempt = Some(Attempt {
                generation,
                validity,
                scope,
            });
        }
        requests
    }

    pub fn not_sent(&mut self, request: &ControlRequest, now: Instant) -> bool {
        let Some(entry) = self.current(request, now) else {
            return false;
        };
        entry.retry(now);
        true
    }

    /// An explicit user retry unblocks observation, never creation delivery.
    pub fn retry_blocked(&mut self, now: Instant) {
        for entry in self.entries.values_mut() {
            if entry.resolution.is_none() && entry.blocked.take().is_some() {
                entry.attempt.take();
                entry.retry_at = now;
            }
        }
    }
}
