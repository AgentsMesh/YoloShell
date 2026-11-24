use crate::{
    fixtures::{constants::*, *},
    server::*,
};
use yolo_hub_identity::{TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{TerminalSummary, TerminalSummaryFields};

#[tokio::test]
async fn client_keeps_mixed_origins_and_carries_the_exact_next_page_cursor() {
    let (mut client, mut server) = connected().await;
    let local = target().address;
    let remote = TerminalAddress {
        hub_id: other_hub(),
        terminal_id: TerminalId::from_uuid(id(SESSION_ID)).unwrap(),
    };
    let summary = |address| {
        TerminalSummary::new(TerminalSummaryFields {
            presentation: Default::default(),
            address,
            name: SAMPLE_PAYLOAD.into(),
            cwd: SOCKET_ROOT.into(),
            archived: false,
            recovery: None,
            runtime: None,
            agent: None,
            workspace: None,
        })
        .unwrap()
    };
    let peer = async {
        let first = receive(&mut server).await;
        assert_eq!(first.fields().command, list());
        respond(
            &mut server,
            &first,
            Reply::Terminals {
                terminals: vec![summary(local), summary(remote)],
                next: Some(remote),
            },
        )
        .await;
        let second = receive(&mut server).await;
        assert_ne!(first.fields().request_id, second.fields().request_id);
        assert_eq!(
            second.fields().command,
            Command::List {
                include_archived: false,
                after: Some(remote)
            }
        );
        respond(
            &mut server,
            &second,
            Reply::Terminals {
                terminals: Vec::new(),
                next: None,
            },
        )
        .await;
    };
    let caller = async {
        let Reply::Terminals { terminals, next } =
            client.call(hub(), list(), deadline()).await.unwrap()
        else {
            panic!()
        };
        assert_eq!(
            terminals
                .iter()
                .map(|session| session.fields().address)
                .collect::<Vec<_>>(),
            vec![local, remote]
        );
        assert_eq!(next, Some(remote));
        let reply = client
            .call(
                hub(),
                Command::List {
                    include_archived: false,
                    after: next,
                },
                deadline(),
            )
            .await
            .unwrap();
        assert!(
            matches!(reply, Reply::Terminals { terminals, next: None } if terminals.is_empty())
        );
    };
    tokio::join!(caller, peer);
}

#[tokio::test]
async fn explicit_destination_is_not_rewritten_to_the_entry_hub_identity() {
    let (mut client, mut server) = connected().await;
    let peer = async {
        let request = receive(&mut server).await;
        assert_eq!(request.fields().destination, other_hub());
        respond(
            &mut server,
            &request,
            Reply::Terminals {
                terminals: Vec::new(),
                next: None,
            },
        )
        .await;
    };
    let (reply, ()) = tokio::join!(client.call(other_hub(), list(), deadline()), peer);
    assert!(matches!(reply, Ok(Reply::Terminals { .. })));
    assert_eq!(client.hello().hub_id, hub());
}
