use super::{
    channel_peer::*,
    chat_support::*,
    input_transport::{Harness, hello_reply},
    interrupt_support,
};
use crate::{
    chat, events,
    network::{Job, Update},
};
use crossterm::event::{KeyCode, KeyModifiers};
use tokio::time::timeout_at;
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseGrant, SecretCapability};
use yolo_tui_chat::{InputReadiness, SendPhase};

const GENERATION_STEP: u64 = 1;

#[tokio::test]
async fn connection_loss_preserves_unknown_chat_delivery_without_consuming_the_unchanged_agent_cursor()
 {
    let (mut state, jobs, mut incoming) = ready();
    let snapshot = chat::agent(&state).unwrap().clone();
    let address = snapshot.fields().terminal.address;
    let terminal = state.terminals[&address].clone();
    let original = state.writers[&address].grant;
    let mut network = Harness::with_grant(original).await;
    state.control_generation = Some(network.generation);
    state.record_grant(original, std::time::Instant::now());
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let queued = sent(&mut incoming);
    let first_id = interrupt_support::request(&queued).fields().request_id;
    network
        .jobs
        .send(Job::Scoped(queued.clone()))
        .await
        .ok()
        .unwrap();
    // The peer received the prompt, but no delivery receipt reaches the client.
    // This is actual uncertain delivery, independent of heartbeat scheduling.
    let dispatched = receive(&mut network.peer).await;
    let Command::AgentControl { request, .. } = &dispatched.fields().command else {
        panic!()
    };
    assert_eq!(request.fields().request_id, first_id);
    network.peer.retire();
    let lost = network.event().await;
    assert!(
        matches!(lost, ControlEvent::Unreachable { generation: Some(previous), .. }
        if previous == network.generation)
    );
    events::apply(&mut state, &jobs, Update::Control(lost));
    assert_eq!(
        chat::current(&state).unwrap().composer.phase(),
        Some(SendPhase::Unknown)
    );
    let not_sent = network.event().await;
    assert!(matches!(&not_sent, ControlEvent::Finished {
        request,
        result: Err(ControlError::NotConnected | ControlError::Client(_)),
        connected: false,
    } if request.command == queued.command));
    events::apply(&mut state, &jobs, Update::Control(not_sent));
    assert_eq!(
        chat::current(&state).unwrap().composer.phase(),
        Some(SendPhase::Unknown)
    );
    assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    assert!(incoming.try_recv().is_err());

    let (stream, _) = timeout_at(deadline(), network.listener.accept())
        .await
        .unwrap()
        .unwrap();
    network.peer = Framed::new(stream);
    let greeting = receive(&mut network.peer).await;
    respond(&mut network.peer, &greeting, hello_reply(address.hub_id)).await;
    let ready = network.event().await;
    assert!(
        matches!(ready, ControlEvent::Ready { generation, .. } if generation != network.generation)
    );
    events::apply(&mut state, &jobs, Update::Control(ready));
    let Job::Refresh { request_id } = incoming.try_recv().unwrap() else {
        panic!()
    };
    network
        .jobs
        .send(Job::Refresh { request_id })
        .await
        .ok()
        .unwrap();
    let (stream, _) = timeout_at(deadline(), network.listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut catalog = Framed::new(stream);
    let greeting = receive(&mut catalog).await;
    respond(&mut catalog, &greeting, hello_reply(address.hub_id)).await;
    let list = receive(&mut catalog).await;
    assert!(matches!(list.fields().command, Command::List { .. }));
    respond(
        &mut catalog,
        &list,
        Reply::Terminals {
            terminals: vec![terminal],
            next: None,
        },
    )
    .await;
    let list = receive(&mut catalog).await;
    assert!(matches!(list.fields().command, Command::ListAgents { .. }));
    respond(
        &mut catalog,
        &list,
        Reply::Agents {
            agents: vec![snapshot.clone()],
            next: None,
        },
    )
    .await;
    let update = timeout_at(deadline(), network.responses.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(&update, Update::Catalog(result) if result.request_id == request_id));
    events::apply(&mut state, &jobs, update);
    let mut fields = *original.fields();
    fields.generation += GENERATION_STEP;
    fields.capability = SecretCapability::generate();
    let fresh = LeaseGrant::new(fields).unwrap();
    state.record_grant(fresh, std::time::Instant::now());
    key(&mut state, &jobs, KeyCode::Char('x'), KeyModifiers::CONTROL);
    chat::synchronize(&mut state);
    assert_eq!(chat::input_readiness(&state), InputReadiness::Ready);
    assert!(incoming.try_recv().is_err());

    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let manual = sent(&mut incoming);
    let request = interrupt_support::request(&manual);
    assert_ne!(request.fields().request_id, first_id);
    assert_eq!(request.fields().expected_cursor, snapshot.cursor());
    assert!(matches!(&manual.command, Command::AgentControl { grant, .. } if *grant == fresh));
    network.jobs.send(Job::Scoped(manual)).await.ok().unwrap();
    let actual = receive(&mut network.peer).await;
    let Command::AgentControl { request, .. } = &actual.fields().command else {
        panic!()
    };
    assert_ne!(request.fields().request_id, first_id);
    assert_eq!(request.fields().expected_cursor, snapshot.cursor());
}
