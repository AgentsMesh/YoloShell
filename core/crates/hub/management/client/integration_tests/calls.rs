use super::{fixture::*, peer::*};
use yolo_hub_management_client::{ManagementError, execute};
use yolo_hub_rpc_protocol::{Command, Method, Reply};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn query_and_configure_use_the_entry_hub_and_validate_exact_receipts() {
    let initial = snapshot();
    let (configure, applied) = change(&initial);
    for (command, snapshot) in [
        (Command::GetConfiguration {}, initial.clone()),
        (configure, applied),
    ] {
        let (_directory, socket, listener) = listener();
        let hub = initial.state.identity.hub();
        let server = async {
            let mut peer = accept(
                &listener,
                hub,
                vec![Method::Hello, Method::GetConfiguration, Method::Configure],
            )
            .await;
            let request = receive(&mut peer).await;
            assert_eq!(request.fields().destination, hub);
            assert_eq!(request.fields().command, command);
            respond(
                &mut peer,
                &request,
                Reply::Configuration {
                    snapshot: snapshot.clone(),
                },
            )
            .await;
            closed(&mut peer).await;
        };
        let (result, ()) = tokio::join!(execute(&socket, hub, command.clone(), deadline()), server);
        assert_eq!(result.unwrap(), snapshot);
        no_retry(&listener).await;
    }
}

#[tokio::test]
async fn lost_or_forged_mutation_receipts_remain_unknown_without_reconnection() {
    for forged in [false, true] {
        let (_directory, socket, listener) = listener();
        let initial = snapshot();
        let hub = initial.state.identity.hub();
        let (command, _) = change(&initial);
        let server = async {
            let mut peer = accept(&listener, hub, vec![Method::Hello, Method::Configure]).await;
            let request = receive(&mut peer).await;
            if forged {
                respond(
                    &mut peer,
                    &request,
                    Reply::Configuration {
                        snapshot: initial.clone(),
                    },
                )
                .await;
            }
        };
        let (result, ()) = tokio::join!(execute(&socket, hub, command, deadline()), server);
        assert_eq!(result, Err(ManagementError::OutcomeUnknown));
        no_retry(&listener).await;
    }
}

#[tokio::test]
async fn unsupported_methods_do_not_deliver_configuration_commands() {
    let (_directory, socket, listener) = listener();
    let initial = snapshot();
    let hub = initial.state.identity.hub();
    let server = async {
        let mut peer = accept(&listener, hub, vec![Method::Hello]).await;
        closed(&mut peer).await;
    };
    let (result, ()) = tokio::join!(
        execute(&socket, hub, Command::GetConfiguration {}, deadline()),
        server
    );
    assert_eq!(result, Err(ManagementError::Unsupported));
    no_retry(&listener).await;
}

#[tokio::test]
async fn revision_conflict_remains_a_rejection_and_never_rewrites_the_request() {
    let (_directory, socket, listener) = listener();
    let initial = snapshot();
    let hub = initial.state.identity.hub();
    let (command, _) = change(&initial);
    let server = async {
        let mut peer = accept(&listener, hub, vec![Method::Hello, Method::Configure]).await;
        let request = receive(&mut peer).await;
        respond(
            &mut peer,
            &request,
            Reply::Error {
                code: PublicError::Conflict,
            },
        )
        .await;
        closed(&mut peer).await;
    };
    let (result, ()) = tokio::join!(execute(&socket, hub, command, deadline()), server);
    assert_eq!(result, Err(ManagementError::Conflict));
    no_retry(&listener).await;
}

#[tokio::test]
async fn cancellation_after_delivery_closes_only_its_own_connection_and_does_not_replay() {
    let (_directory, socket, listener) = listener();
    let initial = snapshot();
    let hub = initial.state.identity.hub();
    let (command, _) = change(&initial);
    let caller = tokio::spawn(async move { execute(&socket, hub, command, deadline()).await });
    let mut peer = accept(&listener, hub, vec![Method::Hello, Method::Configure]).await;
    receive(&mut peer).await;
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    closed(&mut peer).await;
    no_retry(&listener).await;
}
