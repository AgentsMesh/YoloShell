use crate::LocalBackend;
use std::time::Duration;
use tokio::time::Instant;
use yolo_agent_control_contract::NativeControlObservation;
use yolo_conversation_protocol::{
    ConversationBinding, ConversationError, ConversationReadRequest, ConversationReadResult,
    MAX_WATCH_WAIT_MS,
};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

const WATCH_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(crate) mod binding;

impl LocalBackend {
    pub fn with_conversations(
        mut self,
        sources: yolo_conversation_service::ConversationSources,
    ) -> Self {
        self.conversations =
            std::sync::Arc::new(yolo_conversation_service::ConversationReader::new(sources));
        self
    }

    pub(crate) async fn resolve_conversation(
        &self,
        target: ConversationBinding,
    ) -> Result<Reply, PublicError> {
        let before = match self.conversation_binding(target).await {
            Ok(binding) => binding,
            Err(error) => return Ok(unavailable(error)),
        };
        let binding = before.binding().clone();
        let reader = self.conversations.clone();
        let result = tokio::task::spawn_blocking(move || reader.resolve(&binding, target))
            .await
            .map_err(|_| PublicError::Internal)?;
        if let Err(error) = binding::recheck(&before, self.conversation_binding(target).await) {
            return Ok(unavailable(error));
        }
        Ok(match result {
            Ok(conversation) => Reply::Conversation { conversation },
            Err(error) => unavailable(error),
        })
    }

    pub(crate) async fn read_conversation(
        &self,
        request: &ConversationReadRequest,
    ) -> Result<Reply, PublicError> {
        if let Err(error) = request.validate() {
            return Ok(unavailable(error));
        }
        let before = match self.conversation_binding(request.binding).await {
            Ok(binding) => binding,
            Err(error) => return Ok(unavailable(error)),
        };
        let binding = before.binding().clone();
        let request_copy = request.clone();
        let reader = self.conversations.clone();
        let result = tokio::task::spawn_blocking(move || reader.read(&binding, &request_copy))
            .await
            .map_err(|_| PublicError::Internal)?;
        if let Err(error) = binding::recheck(&before, self.conversation_binding(request.binding).await) {
            return Ok(unavailable(error));
        }
        Ok(match result {
            Ok(result) => {
                result
                    .validate_for(request)
                    .map_err(|_| PublicError::Internal)?;
                Reply::ConversationRead { reading: result }
            }
            Err(error) => unavailable(error),
        })
    }

    pub(crate) async fn watch_conversation(
        &self,
        request: &ConversationReadRequest,
        wait_ms: u32,
    ) -> Result<Reply, PublicError> {
        if wait_ms > MAX_WATCH_WAIT_MS {
            return Ok(unavailable(ConversationError::InvalidRequest));
        }
        let deadline = Instant::now() + Duration::from_millis(u64::from(wait_ms));
        loop {
            let reply = self.read_conversation(request).await?;
            if !matches!(
                &reply,
                Reply::ConversationRead {
                    reading: ConversationReadResult::Unchanged { .. }
                }
            ) || Instant::now() >= deadline
            {
                return Ok(reply);
            }
            tokio::time::sleep_until((Instant::now() + WATCH_POLL_INTERVAL).min(deadline)).await;
        }
    }

    /// Both the public execution identity and the live Worker witness must agree.
    /// Callers never provide a path; only the authenticated native binding names evidence.
    async fn conversation_binding(
        &self,
        target: ConversationBinding,
    ) -> Result<NativeControlObservation, ConversationError> {
        target.validate()?;
        if target.agent.address.hub_id != self.hub {
            return Err(ConversationError::BindingChanged);
        }
        let projection = self
            .agents
            .as_ref()
            .ok_or(ConversationError::UnsupportedProvider)?;
        // Evidence participates in the observation order but never holds the
        // projection across Host or Worker I/O; both waits are bounded like a
        // catalogue read.
        let _sweep = self.begin_observation().await;
        let view = self
            .bounded_host_read(
                self.client
                    .get_session(target.runtime.address.terminal_id.as_uuid()),
            )
            .await
            .map_err(|_| ConversationError::EvidenceUnavailable)?;
        let observed = tokio::time::timeout(
            crate::observation::NATIVE_OBSERVATION_TIMEOUT,
            self.capture_native_observation(&view),
        )
        .await
        .ok()
        .flatten()
        .ok_or(ConversationError::EvidenceUnavailable)?;
        let mut projection = projection.lock().await;
        let snapshot = projection
            .project_with_observation(&view, Some(&observed))
            .map_err(|_| ConversationError::EvidenceUnavailable)?
            .ok_or(ConversationError::EvidenceUnavailable)?;
        if snapshot.cursor().target() != target.agent
            || snapshot.fields().terminal != target.runtime
        {
            return Err(ConversationError::BindingChanged);
        }
        Ok(observed)
    }
}

fn unavailable(error: ConversationError) -> Reply {
    Reply::ConversationUnavailable { error }
}
