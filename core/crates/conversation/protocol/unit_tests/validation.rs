use super::*;
use yolo_crypto::Sha256Digest;
use yolo_hub_identity::*;

#[path = "fixture.rs"]
mod fixture;
use fixture::fixture;

#[test]
fn descriptor_is_tied_to_origin_and_both_execution_identities() {
    let (request, page) = fixture();
    let descriptor = page.conversation;
    assert!(descriptor.validate_for(request.binding).is_ok());
    for mutate in [
        (|binding: &mut ConversationBinding| binding.agent.execution_id = ExecutionId::new())
            as fn(&mut ConversationBinding),
        |binding| binding.runtime.runtime_epoch = RuntimeEpoch::new(),
    ] {
        let mut binding = request.binding;
        mutate(&mut binding);
        assert_eq!(
            descriptor.validate_for(binding),
            Err(ConversationError::BindingChanged)
        );
    }
    for native_id in [
        String::new(),
        "bad\nidentity".into(),
        "s".repeat(MAX_PROVIDER_SESSION_ID_BYTES + 1),
    ] {
        let mut changed = descriptor.clone();
        changed.provider_session_id = native_id;
        assert_eq!(
            changed.validate_for(request.binding),
            Err(ConversationError::InvalidRequest)
        );
    }
    let mut descriptor = descriptor;
    descriptor.address.hub_id = HubId::new();
    assert_eq!(
        descriptor.validate_for(request.binding),
        Err(ConversationError::BindingChanged)
    );
    let mut binding = request.binding;
    binding.runtime.address.hub_id = HubId::new();
    assert_eq!(binding.validate(), Err(ConversationError::InvalidRequest));
}

#[test]
fn requests_and_conditional_replies_reject_cross_history_reuse() {
    let (mut request, page) = fixture();
    assert!(request.validate().is_ok());
    let unchanged = ConversationReadResult::Unchanged {
        history_generation: page.history_generation,
        revision: page.revision,
    };
    assert_eq!(
        unchanged.validate_for(&request),
        Err(ConversationError::InvalidRequest)
    );
    request.since_revision = Some(page.revision);
    assert_eq!(request.validate(), Err(ConversationError::InvalidRequest));
    request.since_generation = Some(page.history_generation);
    assert!(unchanged.validate_for(&request).is_ok());
    request.cursor = Some(ConversationCursor {
        address: request.address.clone(),
        history_generation: page.history_generation,
        before_offset: 10,
    });
    assert!(unchanged.validate_for(&request).is_ok());
    request.cursor.as_mut().unwrap().history_generation = Sha256Digest::repeated(9);
    assert_eq!(
        unchanged.validate_for(&request),
        Err(ConversationError::InvalidRequest)
    );
    request.cursor.as_mut().unwrap().address.hub_id = HubId::new();
    assert_eq!(request.validate(), Err(ConversationError::InvalidRequest));
    request.cursor = None;
    request.address.hub_id = HubId::new();
    assert_eq!(request.validate(), Err(ConversationError::InvalidRequest));
}

#[test]
fn pages_enforce_entry_counts_bytes_uniqueness_and_valid_next_cursor() {
    let (request, page) = fixture();
    assert!(
        ConversationReadResult::Page { page: page.clone() }
            .validate_for(&request)
            .is_ok()
    );
    let mutations: [fn(&mut ConversationPage); 8] = [
        |page| page.entries[0].parts.clear(),
        |page| page.entries.push(page.entries[0].clone()),
        |page| page.availability = ConversationAvailability::NotYetWritten,
        |page| {
            page.next_cursor = Some(ConversationCursor {
                address: ConversationAddress {
                    hub_id: HubId::new(),
                    ..page.conversation.address.clone()
                },
                history_generation: page.history_generation,
                before_offset: 1,
            })
        },
        |page| {
            page.next_cursor = Some(ConversationCursor {
                address: page.conversation.address.clone(),
                history_generation: Sha256Digest::repeated(9),
                before_offset: 1,
            })
        },
        |page| {
            page.entries.clear();
            page.next_cursor = Some(ConversationCursor {
                address: page.conversation.address.clone(),
                history_generation: page.history_generation,
                before_offset: 1,
            });
        },
        |page| {
            page.entries[0].parts = vec![ConversationPart::Text {
                text: "x".repeat(MAX_BYTE_LIMIT as usize),
            }]
        },
        |page| {
            page.entries = (0..=MAX_ENTRY_LIMIT)
                .map(|_| page.entries[0].clone())
                .collect()
        },
    ];
    for mutate in mutations {
        let mut altered = page.clone();
        mutate(&mut altered);
        assert_eq!(
            altered.validate_for(&request),
            Err(ConversationError::InvalidRequest)
        );
    }
    let mut altered = page.clone();
    altered.conversation.address.conversation_id = Sha256Digest::repeated(9);
    assert_eq!(
        altered.validate_for(&request),
        Err(ConversationError::BindingChanged)
    );
    let mut request = request;
    request.cursor = Some(ConversationCursor {
        address: request.address.clone(),
        history_generation: page.history_generation,
        before_offset: 20,
    });
    let mut page = page;
    page.next_cursor = Some(ConversationCursor {
        address: request.address.clone(),
        history_generation: page.history_generation,
        before_offset: 10,
    });
    assert!(page.validate_for(&request).is_ok());
    page.next_cursor.as_mut().unwrap().before_offset = 20;
    assert_eq!(
        page.validate_for(&request),
        Err(ConversationError::InvalidRequest)
    );
    page.next_cursor = None;
    request.cursor.as_mut().unwrap().history_generation = Sha256Digest::repeated(9);
    assert_eq!(
        page.validate_for(&request),
        Err(ConversationError::InvalidRequest)
    );
}
