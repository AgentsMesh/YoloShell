use super::fixture::*;
use crate::{
    Effect,
    constants::{CONSUMERS_EMPTY, REFRESH_CONSUMERS},
    model::{Action, Page},
};
use crossterm::event::KeyCode;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::ManagementError;
use yolo_hub_rpc_protocol::{Command, ConsumerSummary, Reply};

fn consumer() -> ConsumerSummary {
    ConsumerSummary {
        hub: HubId::new(),
        connected_since_ms: 1,
        links: 2,
    }
}

#[test]
fn consumers_page_lists_live_importers_after_the_local_query() {
    let mut manager = manager();
    let Some(Effect::Request(request)) = activate(&mut manager, Action::Page(Page::Consumers)) else {
        panic!()
    };
    assert_eq!(request.command, Command::ListConsumers {});
    let consumer = consumer();
    manager.complete(
        request.token,
        Ok(Reply::Consumers {
            consumers: vec![consumer.clone()],
        }),
    );
    assert_eq!(manager.consumers, Some(vec![consumer.clone()]));
    assert_eq!(manager.page, Page::Consumers);
    let labels: Vec<String> = manager.items().into_iter().map(|item| item.label).collect();
    assert!(
        labels
            .iter()
            .any(|label| label.contains(&consumer.hub.to_string()))
    );
    assert!(labels.iter().any(|label| label.contains("2 links")));
    assert!(labels.contains(&REFRESH_CONSUMERS.to_string()));
}

#[test]
fn consumers_page_reports_an_empty_list_and_a_refresh_failure() {
    let mut manager = manager();
    let Some(Effect::Request(first)) = activate(&mut manager, Action::Page(Page::Consumers)) else {
        panic!()
    };
    manager.complete(
        first.token,
        Ok(Reply::Consumers {
            consumers: Vec::new(),
        }),
    );
    let labels: Vec<String> = manager.items().into_iter().map(|item| item.label).collect();
    assert!(labels.iter().any(|label| label.as_str() == CONSUMERS_EMPTY));
    let Some(Effect::Request(second)) = activate(&mut manager, Action::RefreshConsumers) else {
        panic!()
    };
    assert_eq!(second.command, Command::ListConsumers {});
    manager.complete(second.token, Err(ManagementError::Unavailable));
    assert_eq!(manager.consumers, None);
}

#[test]
fn escaping_a_pending_consumers_query_keeps_the_manager_open() {
    let mut manager = manager();
    let Some(Effect::Request(request)) = activate(&mut manager, Action::Page(Page::Consumers)) else {
        panic!()
    };
    assert!(key(&mut manager, KeyCode::Esc).is_none());
    assert!(!manager.pending());
    manager.complete(
        request.token,
        Ok(Reply::Consumers {
            consumers: vec![consumer()],
        }),
    );
    assert_eq!(manager.consumers, None);
}
