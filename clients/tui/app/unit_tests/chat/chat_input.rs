use super::{chat_support::*, drawing_support, interrupt_support};
use crate::{Mode, actions, network::Job};
use crossterm::event::{Event, KeyCode, KeyModifiers};

#[test]
fn chat_switch_retires_queued_pty_scope_without_releasing_the_writer() {
    let (mut state, jobs, mut incoming) = ready();
    let address = state.focused_terminal().unwrap();
    let grant = state.writers[&address].grant;
    assert!(crate::terminal_actions::deliver(
        &mut state,
        &jobs,
        Ok(NATIVE_DRAFT.to_vec())
    ));
    let Job::UserInput(request) = incoming.try_recv().unwrap() else {
        panic!()
    };
    assert!(request.validity.as_ref().unwrap().is_current());
    toggle(&mut state, &jobs);
    assert_eq!(state.mode, Mode::Chat);
    assert!(!request.validity.as_ref().unwrap().is_current());
    assert_eq!(state.writers[&address].grant, grant);
    paste(&mut state, &jobs, PROMPT);
    assert!(incoming.try_recv().is_err());
    assert_eq!(
        crate::chat::current(&state).unwrap().composer.text(),
        PROMPT
    );
    assert!(!crate::terminal_actions::deliver(
        &mut state,
        &jobs,
        Ok(NATIVE_DRAFT.to_vec())
    ));
    key(&mut state, &jobs, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(state.mode, Mode::Terminal);
    toggle(&mut state, &jobs);
    assert_eq!(
        crate::chat::current(&state).unwrap().composer.text(),
        PROMPT
    );
    assert_eq!(state.writers[&address].grant, grant);
}

#[test]
fn composer_editing_is_local_and_renders_explicit_append_semantics_without_resizing_pty() {
    let (mut state, jobs, mut incoming) = ready();
    let before = drawing_support::render(&state, WIDTH, HEIGHT).1;
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::SHIFT);
    paste(&mut state, &jobs, NEXT_PROMPT);
    key(&mut state, &jobs, KeyCode::Left, KeyModifiers::NONE);
    key(&mut state, &jobs, KeyCode::Right, KeyModifiers::NONE);
    assert!(incoming.try_recv().is_err());
    let (text, after) = drawing_support::render(&state, WIDTH, HEIGHT);
    assert_eq!(before.panes, after.panes);
    assert!(text.contains("Append & send"));
    assert!(text.contains("existing terminal input"));
    assert!(text.contains(PROMPT));
    assert!(text.contains(NEXT_PROMPT));
}

#[test]
fn shell_only_stale_catalog_and_changed_execution_do_not_admit_new_chat_input() {
    let (mut state, jobs, mut incoming) = ready();
    state.inventory.current = false;
    toggle(&mut state, &jobs);
    assert_eq!(state.mode, Mode::Terminal);
    state.inventory.current = true;
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    interrupt_support::mutate_agent(&mut state, |fields| {
        fields.capabilities.submit_prompt = yolo_hub_agent_protocol::Capability::Unsupported {};
    });
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    assert!(incoming.try_recv().is_err());
    assert_eq!(
        crate::chat::current(&state).unwrap().composer.text(),
        PROMPT
    );
    let mut state = crate::client_capability_support::state();
    assert!(!actions::handle(
        &mut state,
        &jobs,
        yolo_hub_identity::HubId::new(),
        Event::Key(KeyCode::Esc.into())
    ));
}
