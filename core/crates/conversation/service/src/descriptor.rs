use yolo_agent_contract::AgentBinding;
use yolo_conversation_protocol::*;
use yolo_conversation_source::ConversationSources;
use yolo_crypto::Sha256Digest;

const IDENTITY_DOMAIN: &str = "yolo.conversation.identity.v1";

pub fn resolve_conversation(
    binding: &AgentBinding,
    target: ConversationBinding,
    sources: &ConversationSources,
) -> Result<ConversationDescriptor, ConversationError> {
    if binding.terminal_id != target.runtime.address.terminal_id.as_uuid()
        || target.agent.address.hub_id != target.runtime.address.hub_id
        || binding.provider_session_id.is_empty()
        || binding.data_home.is_empty()
        || binding.evidence_path.is_empty()
    {
        return Err(ConversationError::BindingChanged);
    }
    let provider = sources
        .source_for_kind(&binding.kind)
        .map(|source| source.conversation_token())
        .ok_or(ConversationError::UnsupportedProvider)?;
    let conversation_id = Sha256Digest::capture(&(
        IDENTITY_DOMAIN,
        provider.as_str(),
        &binding.data_home,
        &binding.provider_session_id,
    ))
    .map_err(|_| ConversationError::InvalidRequest)?;
    let descriptor = ConversationDescriptor {
        address: ConversationAddress {
            hub_id: target.agent.address.hub_id,
            conversation_id,
        },
        binding: target,
        provider,
        provider_session_id: binding.provider_session_id.clone(),
        source: ConversationSource::NativeTranscript,
    };
    descriptor.validate_for(target)?;
    Ok(descriptor)
}
