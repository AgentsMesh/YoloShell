use crate::*;

pub const MAX_PROVIDER_SESSION_ID_BYTES: usize = 512;

impl ConversationBinding {
    pub fn validate(self) -> Result<(), ConversationError> {
        if self.agent.address.hub_id != self.runtime.address.hub_id {
            return Err(ConversationError::InvalidRequest);
        }
        Ok(())
    }
}

impl ConversationDescriptor {
    pub fn validate_for(&self, binding: ConversationBinding) -> Result<(), ConversationError> {
        binding.validate()?;
        if self.binding != binding || self.address.hub_id != binding.agent.address.hub_id {
            return Err(ConversationError::BindingChanged);
        }
        if self.provider_session_id.is_empty()
            || self.provider_session_id.len() > MAX_PROVIDER_SESSION_ID_BYTES
            || self.provider_session_id.chars().any(char::is_control)
        {
            return Err(ConversationError::InvalidRequest);
        }
        Ok(())
    }
}

impl ConversationReadRequest {
    pub fn validate(&self) -> Result<(), ConversationError> {
        self.binding.validate()?;
        self.limits.validate()?;
        if self.address.hub_id != self.binding.agent.address.hub_id
            || self.since_revision.is_some() != self.since_generation.is_some()
            || self
                .cursor
                .as_ref()
                .is_some_and(|cursor| cursor.address != self.address)
        {
            return Err(ConversationError::InvalidRequest);
        }
        Ok(())
    }
}

impl ConversationReadResult {
    pub fn validate_for(&self, request: &ConversationReadRequest) -> Result<(), ConversationError> {
        request.validate()?;
        match self {
            Self::Unchanged {
                history_generation,
                revision,
            } => {
                if request.since_revision != Some(*revision)
                    || request.since_generation != Some(*history_generation)
                    || request
                        .cursor
                        .as_ref()
                        .is_some_and(|cursor| cursor.history_generation != *history_generation)
                {
                    return Err(ConversationError::InvalidRequest);
                }
            }
            Self::Page { page } => page.validate_for(request)?,
        }
        Ok(())
    }
}

impl ConversationPage {
    pub fn validate_for(&self, request: &ConversationReadRequest) -> Result<(), ConversationError> {
        request.validate()?;
        self.conversation.validate_for(request.binding)?;
        if self.conversation.address != request.address {
            return Err(ConversationError::BindingChanged);
        }
        let bytes = self
            .entries
            .iter()
            .map(|entry| serde_json::to_vec(entry).map_or(usize::MAX, |bytes| bytes.len()))
            .fold(0usize, usize::saturating_add);
        if self.entries.len() > request.limits.entries as usize
            || bytes > request.limits.bytes as usize
            || self.entries.iter().enumerate().any(|(index, entry)| {
                entry.parts.is_empty()
                    || self.entries[..index]
                        .iter()
                        .any(|other| other.id == entry.id)
            })
            || (self.availability == ConversationAvailability::NotYetWritten
                && !self.entries.is_empty())
            || request
                .cursor
                .as_ref()
                .is_some_and(|cursor| cursor.history_generation != self.history_generation)
            || self.next_cursor.as_ref().is_some_and(|next| {
                next.address != request.address
                    || next.history_generation != self.history_generation
                    || self.entries.is_empty()
                    || request
                        .cursor
                        .as_ref()
                        .is_some_and(|previous| next.before_offset >= previous.before_offset)
            })
        {
            return Err(ConversationError::InvalidRequest);
        }
        Ok(())
    }
}
