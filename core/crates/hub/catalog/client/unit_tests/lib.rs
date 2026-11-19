#[path = "../test_support/mod.rs"]
mod support;
use super::*;
use support::*;
use yolo_hub_rpc_protocol::{Reply, SUPPORTED_METHODS};

#[test]
fn page_extractors_reject_other_response_kinds() {
    assert!(pages::terminals(Reply::Completed {}).is_none());
    assert!(pages::agents(Reply::Completed {}).is_none());
    assert!(pages::hubs(Reply::Completed {}).is_none());
    let catalog = Catalog::default();
    assert!(catalog.terminals.is_empty() && catalog.agents.is_none() && catalog.hubs.is_none());
    assert!(catalog.is_coherent());
    assert!(!CatalogError::Limit.to_string().is_empty());
    assert!(!CatalogError::UnexpectedReply.to_string().is_empty());
    assert!(
        !CatalogError::from(ClientError::Unsupported)
            .to_string()
            .is_empty()
    );
}

#[tokio::test]
async fn unsupported_collections_are_distinct_from_empty_supported_collections() {
    let (mut client, mut server) = connected(vec![Method::Hello, Method::List]).await;
    let (result, ()) = tokio::join!(read(&mut client, deadline()), empty_sessions(&mut server));
    let catalog = result.unwrap();
    assert!(catalog.agents.is_none() && catalog.hubs.is_none());
    let (mut client, mut server) = connected(SUPPORTED_METHODS.to_vec()).await;
    let server = async {
        empty_sessions(&mut server).await;
        let request = receive(&mut server).await;
        assert!(matches!(
            request.fields().command,
            Command::ListAgents { after: None }
        ));
        respond(
            &mut server,
            &request,
            Reply::Agents {
                agents: vec![],
                next: None,
            },
        )
        .await;
        let request = receive(&mut server).await;
        assert!(matches!(
            request.fields().command,
            Command::ListHubs { after: None }
        ));
        respond(
            &mut server,
            &request,
            Reply::Hubs {
                hubs: vec![],
                next: None,
            },
        )
        .await;
    };
    let (result, ()) = tokio::join!(read(&mut client, deadline()), server);
    let catalog = result.unwrap();
    assert_eq!(catalog.agents.unwrap().len(), 0);
    assert_eq!(catalog.hubs.unwrap().len(), 0);
    assert!(client.is_usable());
}

#[tokio::test]
async fn a_mismatched_extractor_returns_a_classified_error() {
    let (mut client, mut server) = connected(vec![Method::Hello, Method::List]).await;
    let (result, ()) = tokio::join!(
        pages::collect(
            &mut client,
            deadline(),
            |after| Command::List {
                include_archived: false,
                after,
            },
            |_reply| -> Option<(Vec<()>, Option<yolo_hub_identity::TerminalAddress>)> { None }
        ),
        empty_sessions(&mut server)
    );
    assert!(matches!(result, Err(CatalogError::UnexpectedReply)));
}

#[tokio::test]
async fn non_terminating_catalog_has_a_bounded_page_count() {
    let (mut client, mut server) =
        connected(vec![Method::Hello, Method::List, Method::ListHubs]).await;
    let server = async {
        empty_sessions(&mut server).await;
        for index in 0..constants::MAX_CATALOG_PAGES {
            let request = receive(&mut server).await;
            let address = hub(FIRST_HUB + index as u128);
            respond(
                &mut server,
                &request,
                Reply::Hubs {
                    hubs: vec![HubSummary {
                        hub_id: address,
                        reachable: true,
                        label: None,
                    }],
                    next: Some(address),
                },
            )
            .await;
        }
    };
    let (result, ()) = tokio::join!(read(&mut client, deadline()), server);
    assert!(matches!(result, Err(CatalogError::Limit)));
}

#[tokio::test]
async fn archived_records_are_requested_only_by_explicit_consumers() {
    let (mut client, mut server) = connected(vec![Method::Hello, Method::List]).await;
    let server = async {
        let request = receive(&mut server).await;
        assert!(matches!(
            request.fields().command,
            Command::List {
                include_archived: true,
                after: None,
            }
        ));
        respond(
            &mut server,
            &request,
            Reply::Terminals {
                terminals: vec![],
                next: None,
            },
        )
        .await;
    };
    let (result, ()) = tokio::join!(read_with_archived(&mut client, deadline(), true), server);
    let catalog = result.unwrap();
    assert!(catalog.agents.is_none() && catalog.hubs.is_none());
}
