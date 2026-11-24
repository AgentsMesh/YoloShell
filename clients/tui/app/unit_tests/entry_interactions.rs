use super::{query_support::Fixture, startup_support};
use crate::{Mode, actions, help_constants::*, network::Job};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use yolo_hub_rpc_protocol::Command;

fn key(f: &mut Fixture, code: KeyCode, modifiers: KeyModifiers) -> bool {
    actions::handle(
        &mut f.state,
        &f.jobs,
        f.target.address.hub_id,
        Event::Key(KeyEvent::new(code, modifiers)),
    )
}
fn prefix(f: &mut Fixture, command: char) -> bool {
    assert!(!key(f, KeyCode::Char('b'), KeyModifiers::CONTROL));
    key(f, KeyCode::Char(command), KeyModifiers::NONE)
}

#[test]
fn ordinary_q_and_question_mark_are_pty_input_while_detach_never_terminates() {
    for character in [QUIT_KEY, HELP_KEY] {
        let mut f = Fixture::new();
        f.state.mode = Mode::Terminal;
        assert!(!key(&mut f, KeyCode::Char(character), KeyModifiers::NONE));
        let Job::UserInput(request) = f.incoming.try_recv().unwrap() else {
            panic!("expected terminal input")
        };
        let Command::Input { request } = request.command else {
            panic!("expected input")
        };
        assert_eq!(request.fields().data, character.to_string().as_bytes());
        f.no_jobs();
    }
    for command in [DETACH_KEY, QUIT_KEY] {
        let mut f = Fixture::new();
        f.state.mode = Mode::Terminal;
        assert!(prefix(&mut f, command));
        f.no_jobs();
        assert!(f.state.writers.contains_key(&f.target.address));
        assert_eq!(f.state.focused_terminal(), Some(f.target.address));
    }
}

#[test]
fn help_captures_input_cancels_queued_input_and_preserves_terminal_binding() {
    let mut f = Fixture::new();
    f.state.mode = Mode::Terminal;
    key(&mut f, KeyCode::Char(QUIT_KEY), KeyModifiers::NONE);
    let Job::UserInput(queued) = f.incoming.try_recv().unwrap() else {
        panic!("expected input")
    };
    let binding = f.state.focused_terminal();
    assert!(!prefix(&mut f, HELP_KEY));
    assert!(f.state.help);
    assert!(!queued.validity.unwrap().is_current());
    key(&mut f, KeyCode::Char(NEW_SHELL_KEY), KeyModifiers::NONE);
    f.no_jobs();
    key(&mut f, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!f.state.help);
    assert_eq!(f.state.mode, Mode::Terminal);
    assert_eq!(f.state.focused_terminal(), binding);
}

#[test]
fn session_list_and_new_shell_have_distinct_keys_and_reuse_origin_directory() {
    let mut f = Fixture::new();
    f.state.mode = Mode::Terminal;
    prefix(&mut f, SESSIONS_KEY);
    assert_eq!(f.state.mode, Mode::Inventory);
    f.no_jobs();
    prefix(&mut f, NEW_SHELL_KEY);
    let Job::Call {
        origin,
        command: Command::Create { request },
        ..
    } = f.incoming.try_recv().unwrap()
    else {
        panic!("expected shell creation")
    };
    assert_eq!(origin, f.target.address.hub_id);
    assert_eq!(request.fields().cwd, super::fixtures::SESSION_DIRECTORY);
    assert!(f.state.writers.contains_key(&f.target.address));
}

#[test]
fn fresh_directory_form_uses_configured_cwd_and_remote_scope_uses_host_home() {
    let mut f = startup_support::Fixture::automatic();
    crate::actions::handle(
        &mut f.state,
        &f.jobs,
        f.hub,
        Event::Key(KeyCode::Char('n').into()),
    );
    assert_eq!(f.state.entry, startup_support::DIRECTORY);
    let remote = yolo_hub_identity::HubId::new();
    f.state.tabs.select_machine(remote).unwrap();
    f.state.inventory.selected_machine = Some(remote);
    f.state.mode = Mode::Inventory;
    crate::actions::handle(
        &mut f.state,
        &f.jobs,
        f.hub,
        Event::Key(KeyCode::Char('n').into()),
    );
    assert_eq!(f.state.entry, crate::constants::DEFAULT_DIRECTORY);
}

#[test]
fn empty_screen_exposes_creation_and_help_without_false_read_only_or_workspace_label() {
    let mut f = startup_support::Fixture::automatic();
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| {
            crate::draw::draw(frame, &f.state);
        })
        .unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(text.contains(crate::constants::EMPTY_INVENTORY_HINT));
    assert!(!text.contains(crate::constants::READ_ONLY_LABEL));
    assert!(text.contains(yolo_tui_tab::DEFAULT_TAB_NAME));
    assert!(!text.contains("LocalTabView"));
    crate::help::open(&mut f.state);
    terminal
        .draw(|frame| {
            crate::draw::draw(frame, &f.state);
        })
        .unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(text.contains("Start claude, codex"));
    assert!(text.contains("Detach Yolo"));
}

#[test]
fn tab_manager_and_cycle_prefixes_preserve_terminal_binding_without_creating_shells() {
    let mut f = Fixture::new();
    f.state.mode = Mode::Terminal;
    let layout = f.state.tab_layout().clone();
    assert!(!prefix(&mut f, crate::tabs::MANAGER_KEY));
    assert!(f.state.tab_manager.is_some());
    key(&mut f, KeyCode::Esc, KeyModifiers::NONE);
    assert!(f.state.tab_manager.is_none());
    for command in ['N', ',', crate::tabs::NEXT_KEY, crate::tabs::PREVIOUS_KEY] {
        assert!(!prefix(&mut f, command));
    }
    assert!(f.state.tab_manager.is_none());
    assert_eq!(*f.state.tab_layout(), layout);
    f.no_jobs();
}
