use crate::cache::CachedPage;
use crate::{descriptor::resolve_conversation, evidence, prefix, registry::Registry};
use std::sync::Mutex;
use yolo_agent_contract::AgentBinding;
use yolo_conversation_protocol::*;
use yolo_conversation_source::{ConversationReadOutcome, ConversationSources, conversation_error};
use yolo_conversation_transcript::scan;
use yolo_crypto::Sha256Digest;

const REVISION_DOMAIN: &str = "yolo.conversation.page.v1";

pub struct ConversationReader {
    histories: Mutex<Registry>,
    sources: ConversationSources,
}

impl ConversationReader {
    pub fn new(sources: ConversationSources) -> Self {
        Self {
            histories: Mutex::default(),
            sources,
        }
    }

    pub fn resolve(
        &self,
        binding: &AgentBinding,
        target: ConversationBinding,
    ) -> Result<ConversationDescriptor, ConversationError> {
        let descriptor = resolve_conversation(binding, target, &self.sources)?;
        self.sources
            .source_for_token(&descriptor.provider)
            .ok_or(ConversationError::UnsupportedProvider)?
            .verify(binding)?;
        Ok(descriptor)
    }

    pub fn read(
        &self,
        binding: &AgentBinding,
        request: &ConversationReadRequest,
    ) -> Result<ConversationReadResult, ConversationError> {
        request.validate()?;
        let descriptor = resolve_conversation(binding, request.binding, &self.sources)?;
        if descriptor.address != request.address {
            return Err(ConversationError::BindingChanged);
        }
        let source = self
            .sources
            .source_for_token(&descriptor.provider)
            .ok_or(ConversationError::UnsupportedProvider)?;
        let parser = source.parser();
        let context = self
            .histories
            .lock()
            .map_err(|_| ConversationError::EvidenceUnavailable)?
            .lease(&request.address)?;
        let mut history = context
            .lock()
            .map_err(|_| ConversationError::EvidenceUnavailable)?;
        let previous_length = history
            .get(&request.address)
            .map_or(0, |entry| entry.length);
        let cache_key = Sha256Digest::capture(&(&descriptor, &request.cursor, request.limits))
            .map_err(|_| ConversationError::InvalidRequest)?;
        let read = source.with_evidence(binding, &mut |file| {
            prefix::guard(file.file(), file.length(), |file, length| {
                let stamp = evidence::snapshot_stamp(file, length)?;
                if let Some(cache) = history
                    .get(&request.address)
                    .and_then(|entry| entry.cache.as_ref())
                    .filter(|cache| Some(cache.stamp) == stamp && cache.key == cache_key)
                {
                    let digest = history
                        .get(&request.address)
                        .map(|entry| entry.digest.clone());
                    return Ok((
                        ConversationReadOutcome::Cached(
                            cache.read(request.since_revision, request.since_generation),
                        ),
                        digest,
                    ));
                }
                let before = request
                    .cursor
                    .as_ref()
                    .map_or(length, |cursor| cursor.before_offset);
                let object = evidence::object(file)?;
                let snapshot = scan(
                    evidence::reader(file)?,
                    length,
                    previous_length,
                    before,
                    request.limits,
                    parser,
                )?;
                let digest = snapshot.digest.clone();
                Ok((
                    ConversationReadOutcome::Scanned {
                        object,
                        stamp,
                        snapshot,
                    },
                    Some(digest),
                ))
            })
            .map_err(conversation_error)
        })?;
        let (object, stamp, snapshot) = match read {
            ConversationReadOutcome::Cached(result) => return Ok(result),
            ConversationReadOutcome::Scanned {
                object,
                stamp,
                snapshot,
            } => (object, stamp, snapshot),
        };
        let generation = history.record(&request.address, object, &snapshot);
        if request.cursor.as_ref().is_some_and(|cursor| {
            cursor.history_generation != generation
                || cursor.before_offset > snapshot.length
                || cursor.before_offset < snapshot.selection.floor
        }) {
            return Err(ConversationError::HistoryChanged);
        }
        let revision = Sha256Digest::capture(&(
            REVISION_DOMAIN,
            &descriptor,
            generation,
            &snapshot.digest,
            &request.cursor,
            request.limits,
        ))
        .map_err(|_| ConversationError::InvalidRequest)?;
        if request.since_revision == Some(revision) && request.since_generation == Some(generation)
        {
            return Ok(ConversationReadResult::Unchanged {
                history_generation: generation,
                revision,
            });
        }
        let next_cursor = snapshot
            .selection
            .entries
            .front()
            .filter(|_| snapshot.selection.has_older)
            .map(|(offset, _)| ConversationCursor {
                address: request.address.clone(),
                history_generation: generation,
                before_offset: *offset,
            });
        let has_content = snapshot.selection.has_content;
        let entries = snapshot
            .selection
            .entries
            .into_iter()
            .map(|(offset, entry)| entry.project(generation, offset))
            .collect();
        let page = ConversationPage {
            conversation: descriptor,
            history_generation: generation,
            revision,
            availability: if !has_content && snapshot.selection.partial.is_empty() {
                ConversationAvailability::NotYetWritten
            } else {
                ConversationAvailability::Available
            },
            partial: snapshot.selection.partial,
            entries,
            next_cursor,
        };
        if let Some(stamp) = stamp {
            history.cache_page(
                &request.address,
                CachedPage {
                    stamp,
                    key: cache_key,
                    page: page.clone(),
                },
            );
        }
        Ok(ConversationReadResult::Page { page })
    }
}
