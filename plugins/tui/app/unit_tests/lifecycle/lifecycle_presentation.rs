use super::{drawing_support, fixtures};
use crate::{Mode, State};
use tokio::sync::mpsc;
use yolo_hub_agent_protocol::{Activity, AgentSnapshot, Lifecycle};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::{
    OutputCursor, RuntimeStatus, TerminalEvent, TerminalSnapshot, TerminalSnapshotFields,
    TerminalSummary,
};
use yolo_tui_inventory::ControlBadge;
use yolo_tui_lifecycle::*;
use yolo_tui_terminal::TerminalSurface;

const WIDTH: u16 = 120;
const HEIGHT: u16 = 24;
const OUTPUT: &[u8] = b"last terminal screen retained\r\n";
const OUTPUT_TEXT: &str = "last terminal screen retained";
const RAW_CONFLICT: &str = "Hub request was rejected: operation conflicts with current state";

fn running() -> (State, TerminalAddress, String) {
    let (agent, terminal) = fixtures::agent();
    let provider = agent.fields().provider.kind().to_owned();
    let address = terminal.fields().address;
    let mut state = crate::client_capability_support::state();
    state.mode = Mode::Terminal;
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.inventory.replace(Some(vec![agent]), Some(vec![]));
    state.inventory.current = true;
    state.replace_terminals(vec![terminal.clone()]);
    state.bind_focused(Some(address)).unwrap();
    state.record_grant(fixtures::grant(&terminal), std::time::Instant::now());
    let (_, presentation) = drawing_support::render(&state, WIDTH, HEIGHT);
    let size = drawing_support::size(&presentation, address);
    drawing_support::synchronize(&mut state, address, size);
    state.surfaces.insert(
        address,
        TerminalSurface::new(
            &TerminalSnapshot::new(TerminalSnapshotFields {
                target: terminal.fields().runtime.unwrap().target,
                barrier_sequence: u64::default(),
                size,
                data: OUTPUT.into(),
            })
            .unwrap(),
        ),
    );
    (state, address, provider)
}

fn rendered(state: &State) -> String {
    drawing_support::render(state, WIDTH, HEIGHT).0
}

fn status(state: &State) -> String {
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal
        .draw(|frame| {
            crate::draw::draw(frame, state);
        })
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .chunks(usize::from(WIDTH))
        .last()
        .unwrap()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn completed_agent_does_not_replace_the_live_shells_status() {
    let (mut state, address, provider) = running();
    assert!(rendered(&state).contains(&provider));
    let id = state.terminals[&address].fields().agent.unwrap();
    let target = id;
    let mut fields = state.inventory.agents[&target].fields().clone();
    fields.lifecycle = Lifecycle::Exited {
        exit_code: Some(i32::default()),
    };
    fields.activity = Activity::Idle;
    state
        .inventory
        .agents
        .insert(target, AgentSnapshot::new(fields).unwrap());
    let screen = rendered(&state);
    assert!(!screen.contains(&provider));
    assert!(screen.contains(WRITING_LABEL));
    assert!(screen.contains(OUTPUT_TEXT));
    assert_eq!(
        crate::terminal_lifecycle::phase(&state, address),
        TerminalPhase::Running
    );
}

#[test]
fn confirmed_shell_exit_retires_the_pane_and_returns_to_inventory() {
    let (mut state, address, provider) = running();
    let target = state.terminals[&address].fields().runtime.unwrap().target;
    crate::observation_events::terminal(
        &mut state,
        TerminalEvent::Exited {
            cursor: OutputCursor {
                target,
                after_sequence: u64::default(),
            },
            code: Some(i32::default()),
            signal: None,
        },
    );
    let screen = rendered(&state);
    assert_eq!(state.focused_terminal(), None);
    assert_eq!(state.mode, Mode::Inventory);
    assert!(state.sidebar_visible);
    assert!(state.ended_epoch(address).is_some());
    assert!(!screen.contains(OUTPUT_TEXT));
    assert!(!screen.contains(ENDED_ACTION));
    let footer = status(&state);
    assert!(!footer.contains(&provider));
    assert!(!footer.contains(WRITING_LABEL));
    assert!(state.grant(address, std::time::Instant::now()).is_none());
}

#[test]
fn blocked_output_has_context_instead_of_offline_and_raw_conflict() {
    let (mut state, address, _) = running();
    let mut fields = state.terminals[&address].fields().clone();
    fields.runtime.as_mut().unwrap().status = RuntimeStatus::OutputBlocked;
    state
        .terminals
        .insert(address, TerminalSummary::new(fields).unwrap());
    state.status = RAW_CONFLICT.into();
    state.mark_offline(address);
    let screen = rendered(&state);
    assert!(screen.contains(OUTPUT_BLOCKED_LABEL));
    assert!(screen.contains(OUTPUT_BLOCKED_ACTION));
    assert!(screen.contains(OUTPUT_TEXT));
    assert!(!screen.contains("Offline"));
    assert!(!screen.contains(RAW_CONFLICT));
    assert!(!status(&state).contains(WRITING_LABEL));
}

#[test]
fn channel_loss_keeps_runtime_and_screen_without_claiming_agent_exit() {
    let (mut state, address, provider) = running();
    let target = state.terminals[&address].fields().runtime.unwrap().target;
    crate::observation_events::closed(&mut state, target, None);
    let screen = rendered(&state);
    assert!(screen.contains(TERMINAL_RECONNECTING_LABEL));
    assert!(screen.contains(OUTPUT_TEXT));
    assert!(!screen.contains(ENDED_LABEL));
    assert!(!status(&state).contains(&provider));
    assert!(!state.ended_epoch(address).is_some());
    assert_eq!(
        crate::terminal_lifecycle::phase(&state, address),
        TerminalPhase::Running
    );
}

#[test]
fn initial_control_is_syncing_until_its_writer_grant_arrives() {
    let (mut state, address, _) = running();
    state.writers.clear();
    let (jobs, _incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    crate::startup_attach::open(&mut state, &jobs, address);
    let projection = crate::terminal_lifecycle::projection(&state, address, None);
    assert_eq!(projection.input, InputAccess::Synchronizing);
    assert_eq!(crate::control::detail(&state, address), SYNCING_LABEL);
    assert_eq!(
        crate::control::badge(&state, address),
        ControlBadge::Syncing
    );
    assert_eq!(
        crate::control::marker(&state, address),
        Some(ControlBadge::Syncing.glyph())
    );
    let screen = rendered(&state);
    assert!(screen.contains(SYNCING_LABEL));
    assert!(!screen.contains(READ_ONLY_LABEL));
    assert!(!screen.contains("take control"));
}

#[test]
fn hub_loss_is_distinct_from_a_terminal_or_agent_exit() {
    let (mut state, address, _) = running();
    state.disconnected();
    let screen = rendered(&state);
    assert!(screen.contains(HUB_RECONNECTING_LABEL));
    assert!(screen.contains(OUTPUT_TEXT));
    assert!(!screen.contains(ENDED_LABEL));
    assert!(!state.ended_epoch(address).is_some());
}
