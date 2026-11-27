use super::{backend::*, model::*};
use yolo_hub_client_access_dispatch::ScopedDispatcher;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{Command, HubSummary, MAX_CATALOG_PAGE_TERMINALS, Reply};

#[tokio::test]
async fn scoped_session_pages_cross_hidden_origins_without_returning_their_cursor() {
    let (_registry, admission) = authority(&[Right::Observe]);
    let hidden = summary(HIDDEN, SESSION);
    let visible = summary(ORIGIN, SESSION);
    let backend = Scripted::new([
        Reply::Terminals {
            next: Some(hidden.fields().address),
            terminals: vec![hidden],
        },
        Reply::Terminals {
            terminals: vec![visible.clone()],
            next: None,
        },
    ]);
    let scoped = ScopedDispatcher::new(backend.clone(), admission).unwrap();
    let actual = reply(
        scoped
            .dispatch(request(
                ENTRY,
                Command::List {
                    include_archived: true,
                    after: None,
                },
            ))
            .await,
    );
    assert_eq!(
        actual,
        Reply::Terminals {
            terminals: vec![visible],
            next: None
        }
    );
    assert_eq!(backend.calls.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn full_visible_page_returns_only_its_own_cursor_when_another_visible_row_exists() {
    let (_registry, admission) = authority(&[Right::Observe]);
    let visible = (1..=MAX_CATALOG_PAGE_TERMINALS)
        .map(|index| summary(ORIGIN, index as u128))
        .collect::<Vec<_>>();
    let after = visible.last().unwrap().fields().address;
    let backend = Scripted::new([
        Reply::Terminals {
            terminals: visible.clone(),
            next: Some(after),
        },
        Reply::Terminals {
            terminals: vec![summary(ORIGIN, MAX_CATALOG_PAGE_TERMINALS as u128 + 1)],
            next: None,
        },
    ]);
    let scoped = ScopedDispatcher::new(backend, admission).unwrap();
    assert_eq!(
        reply(
            scoped
                .dispatch(request(
                    ENTRY,
                    Command::List {
                        include_archived: false,
                        after: None
                    }
                ))
                .await
        ),
        Reply::Terminals {
            terminals: visible,
            next: Some(after)
        }
    );
}

#[tokio::test]
async fn scopes_without_observe_cannot_read_aggregate_inventory() {
    let (_registry, admission) = authority(&[Right::TerminalRead]);
    let backend = Scripted::new([
        Reply::Hubs {
            hubs: vec![HubSummary {
                hub_id: hub(ORIGIN),
                reachable: true,
                label: None,
            }],
            next: None,
        },
        Reply::Terminals {
            terminals: vec![summary(ORIGIN, SESSION)],
            next: None,
        },
        Reply::Agents {
            agents: vec![],
            next: None,
        },
    ]);
    let scoped = ScopedDispatcher::new(backend, admission).unwrap();
    assert_eq!(
        reply(
            scoped
                .dispatch(request(ENTRY, Command::ListHubs { after: None }))
                .await
        ),
        Reply::Hubs {
            hubs: vec![],
            next: None
        }
    );
    assert_eq!(
        reply(
            scoped
                .dispatch(request(
                    ENTRY,
                    Command::List {
                        include_archived: false,
                        after: None
                    }
                ))
                .await
        ),
        Reply::Terminals {
            terminals: vec![],
            next: None
        }
    );
    assert_eq!(
        reply(
            scoped
                .dispatch(request(ENTRY, Command::ListAgents { after: None }))
                .await
        ),
        Reply::Agents {
            agents: vec![],
            next: None
        }
    );
}
