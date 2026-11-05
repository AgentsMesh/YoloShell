use crate::{Command, Method, Reply};
use yolo_hub_identity::HubId;

pub(crate) fn valid(reply: &Reply, method: Method, origin: HubId) -> Option<bool> {
    use Method::*;
    if !matches!(
        method,
        ListLayouts
            | GetLayout
            | UpdateLayout
            | ResolveConversation
            | ReadConversation
            | WatchConversation
    ) {
        return None;
    }
    Some(match (reply, method) {
        (Reply::Error { .. }, _) => true,
        (Reply::Layouts { documents }, ListLayouts) => {
            documents.len() <= yolo_presentation_protocol::MAX_DOCUMENTS
                && documents.iter().all(|document| {
                    document.address.hub_id == origin && document.validate().is_ok()
                })
                && documents
                    .windows(2)
                    .all(|pair| pair[0].address < pair[1].address)
        }
        (Reply::Layout { document }, GetLayout | UpdateLayout) => {
            (method != UpdateLayout || document.is_some())
                && document.as_ref().is_none_or(|document| {
                    document.address.hub_id == origin && document.validate().is_ok()
                })
        }
        (Reply::Conversation { conversation }, ResolveConversation) => {
            conversation.address.hub_id == origin
        }
        (Reply::ConversationRead { .. }, ReadConversation | WatchConversation) => true,
        (
            Reply::ConversationUnavailable { .. },
            ResolveConversation | ReadConversation | WatchConversation,
        ) => true,
        _ => false,
    })
}

pub(crate) fn matches_target(reply: &Reply, command: &Command) -> Option<bool> {
    Some(match (reply, command) {
        (Reply::Layout { document }, Command::GetLayout { address }) => document
            .as_ref()
            .is_none_or(|document| document.address == *address),
        (
            Reply::Layout {
                document: Some(document),
            },
            Command::UpdateLayout { change },
        ) => {
            document.address == change.address
                && document.last_request_id == change.request_id
                && change
                    .expected_revision
                    .checked_add(yolo_presentation_protocol::FIRST_REVISION)
                    == Some(document.revision)
                && document.tabs == change.tabs
        }
        (Reply::Conversation { conversation }, Command::ResolveConversation { binding }) => {
            conversation.validate_for(*binding).is_ok()
        }
        (
            Reply::ConversationRead { reading },
            Command::ReadConversation { request } | Command::WatchConversation { request, .. },
        ) => reading.validate_for(request).is_ok(),
        _ => return None,
    })
}
