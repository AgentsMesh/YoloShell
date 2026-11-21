#[path = "../test_support/mod.rs"]
mod support;
use support::*;
use yolo_hub_catalog_client::{CatalogError, read};
use yolo_hub_rpc_protocol::{Command, HubSummary, Method, Reply};

#[tokio::test]
async fn multi_page_hubs_retain_exact_cursor_and_origin_order() {
    let (mut client, mut server) =
        connected(vec![Method::Hello, Method::List, Method::ListHubs]).await;
    let first = hub(FIRST_HUB);
    let second = hub(NEXT_HUB);
    let server = async {
        empty_sessions(&mut server).await;
        let request = receive(&mut server).await;
        assert!(matches!(
            request.fields().command,
            Command::ListHubs { after: None }
        ));
        respond(
            &mut server,
            &request,
            Reply::Hubs {
                hubs: vec![HubSummary {
                    hub_id: first,
                    reachable: true,
                    label: None,
                }],
                next: Some(first),
            },
        )
        .await;
        let request = receive(&mut server).await;
        assert_eq!(
            request.fields().command,
            Command::ListHubs { after: Some(first) }
        );
        respond(
            &mut server,
            &request,
            Reply::Hubs {
                hubs: vec![HubSummary {
                    hub_id: second,
                    reachable: false,
                    label: None,
                }],
                next: None,
            },
        )
        .await;
    };
    let (result, ()) = tokio::join!(read(&mut client, deadline()), server);
    assert_eq!(
        result.unwrap().hubs.unwrap(),
        vec![
            HubSummary {
                hub_id: first,
                reachable: true,
                label: None,
            },
            HubSummary {
                hub_id: second,
                reachable: false,
                label: None,
            },
        ]
    );
}

#[tokio::test]
async fn a_repeated_page_is_rejected_without_publishing_a_partial_catalog() {
    let (mut client, mut server) =
        connected(vec![Method::Hello, Method::List, Method::ListHubs]).await;
    let first = hub(FIRST_HUB);
    let server = async {
        empty_sessions(&mut server).await;
        let request = receive(&mut server).await;
        let reply = Reply::Hubs {
            hubs: vec![HubSummary {
                hub_id: first,
                reachable: true,
                label: None,
            }],
            next: Some(first),
        };
        respond(&mut server, &request, reply.clone()).await;
        let request = receive(&mut server).await;
        respond(&mut server, &request, reply).await;
    };
    let (result, ()) = tokio::join!(read(&mut client, deadline()), server);
    assert!(matches!(result, Err(CatalogError::Client(_))));
    assert!(!client.is_usable());
}

#[tokio::test]
async fn cancellation_mid_refresh_retires_the_channel_and_discards_partial_results() {
    let (mut client, mut server) =
        connected(vec![Method::Hello, Method::List, Method::ListHubs]).await;
    let (sent, received) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        empty_sessions(&mut server).await;
        let request = receive(&mut server).await;
        assert!(matches!(request.fields().command, Command::ListHubs { .. }));
        sent.send(()).unwrap();
        server
    });
    let mut pending = Box::pin(read(&mut client, deadline()));
    tokio::select! {
        _ = &mut pending => panic!(),
        received = received => received.unwrap(),
    }
    drop(pending);
    assert!(!client.is_usable());
    drop(server.await.unwrap());
}
