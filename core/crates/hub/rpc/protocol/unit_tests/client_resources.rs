use crate::commands::{request, resources::*};
use crate::fixtures::{constants::*, hub, id};
use yolo_conversation_protocol::*;
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{Command, Method, Reply, Request, Response};
use yolo_hub_terminal_protocol::PublicError;

const NEXT_REVISION: u64 = 1;

#[test]
fn resource_reads_require_egress_admission_but_layout_mutations_keep_effect_semantics() {
    for (command, _) in samples() {
        assert_eq!(command.requires_read_reply_admission(), !command.mutates());
    }
}

#[test]
fn content_and_layout_rights_do_not_imply_terminal_or_agent_control() {
    for (method, right) in [
        (Method::ResolveConversation, Right::ConversationRead),
        (Method::ReadConversation, Right::ConversationRead),
        (Method::WatchConversation, Right::ConversationRead),
        (Method::ListLayouts, Right::PresentationRead),
        (Method::GetLayout, Right::PresentationRead),
        (Method::UpdateLayout, Right::PresentationWrite),
    ] {
        let rights = method.required_rights().unwrap();
        assert_eq!(rights, [right].into_iter().collect());
        for denied in [
            Right::TerminalWrite,
            Right::AgentControl,
            Right::Lifecycle,
            Right::FileRead,
        ] {
            assert!(!rights.allows(&[denied].into_iter().collect()));
        }
    }
}

#[test]
fn malformed_read_bindings_and_watch_budgets_never_enter_transport() {
    let mut reading = conversation_read();
    reading.binding.runtime.address.hub_id = hub(SECOND_HUB_ID);
    assert_eq!(
        Request::new(hub(HUB_ID), Command::ReadConversation { request: reading }),
        Err(PublicError::InvalidRequest)
    );
    assert_eq!(
        Request::new(
            hub(HUB_ID),
            Command::WatchConversation {
                request: conversation_read(),
                wait_ms: MAX_WATCH_WAIT_MS + NEXT_REVISION as u32,
            }
        ),
        Err(PublicError::InvalidRequest)
    );
    let mut change = layout_update();
    change.address.layout_id = uuid::Uuid::nil();
    assert_eq!(
        Request::new(
            hub(HUB_ID),
            Command::GetLayout {
                address: change.address
            }
        ),
        Err(PublicError::InvalidRequest)
    );
    change.address.layout_id = id(SESSION_ID);
    change.expected_revision = u64::MAX;
    assert_eq!(
        Request::new(hub(HUB_ID), Command::UpdateLayout { change }),
        Err(PublicError::InvalidRequest)
    );
}

#[test]
fn layout_mutation_reply_must_identify_exact_change_and_revision() {
    let change = layout_update();
    let current = change.apply(None).unwrap();
    let request = request(Command::UpdateLayout { change });
    assert!(
        Response::new(
            &request,
            hub(HUB_ID),
            Reply::Layout {
                document: Some(current.clone())
            }
        )
        .validate_for(&request)
        .is_ok()
    );
    let mut mutations = vec![current.clone(), current.clone(), current.clone()];
    mutations[0].last_request_id = id(SECOND_SESSION_ID);
    mutations[1].revision += NEXT_REVISION;
    mutations[2].address.hub_id = hub(SECOND_HUB_ID);
    for document in mutations {
        assert_eq!(
            Response::new(
                &request,
                hub(HUB_ID),
                Reply::Layout {
                    document: Some(document)
                }
            )
            .validate_for(&request),
            Err(PublicError::InvalidPayload)
        );
    }
    assert_eq!(
        Response::new(&request, hub(HUB_ID), Reply::Layout { document: None })
            .validate_for(&request),
        Err(PublicError::InvalidPayload)
    );
    let list = request_from(Command::ListLayouts {});
    let summary = yolo_presentation_protocol::LayoutSummary::try_from(&current).unwrap();
    assert_eq!(
        Response::new(
            &list,
            hub(HUB_ID),
            Reply::Layouts {
                documents: vec![summary, summary]
            }
        )
        .validate_for(&list),
        Err(PublicError::InvalidPayload)
    );
}

fn request_from(command: Command) -> Request {
    request(command)
}
