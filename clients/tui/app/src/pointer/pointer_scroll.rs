use crate::{State, network::Job};
use crossterm::event::MouseEventKind;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use yolo_hub_agent_protocol::PROVIDER_CODEX;
use yolo_hub_identity::TerminalAddress;

const PAGE_UP: &[u8] = b"\x1b[5~";
const PAGE_DOWN: &[u8] = b"\x1b[6~";
const LINE_UP: &[u8] = b"\x1b[A";
const LINE_DOWN: &[u8] = b"\x1b[B";
/// Codex opens its pager overlay with Ctrl-T. The overlay is the only view
/// that exposes the conversation history: the main prompt uses a scroll
/// region, so its transcript never reaches terminal scrollback.
const CODEX_OPEN_TRANSCRIPT: &[u8] = b"\x14";
const CODEX_TRANSCRIPT_MARKER: &str = "T R A N S C R I P T";
const CODEX_TRANSCRIPT_HINT: &str = "pgup/pgdn to page";
const CODEX_COMPOSER_PROMPT: &str = "›";
const CODEX_MODEL_FOOTER: &str = " · ";
/// Codex's prompt, draft lines and footer occupy the bottom of the screen.
/// Leave room for a multi-line draft while still excluding an earlier frame
/// that the shell prompt has already moved past.
const CODEX_COMPOSER_ROWS: u16 = 12;
/// A page key moves a whole screen and reads as a jump. Codex's pager exposes
/// one-line arrows, so spend a conventional wheel notch on a few lines.
const CODEX_SCROLL_LINES: usize = 3;
/// Codex renders the overlay asynchronously after Ctrl-T. A fast trackpad
/// burst can carry more wheel events before that redraw is visible; sending
/// another Ctrl-T would close the overlay and produce the observed flicker.
const CODEX_OPEN_COOLDOWN: Duration = Duration::from_millis(250);

struct CodexWheel {
    keys: Vec<u8>,
    opened_transcript: bool,
    transcript_visible: bool,
}

pub fn handle(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    address: Option<TerminalAddress>,
    event: MouseEventKind,
) -> bool {
    if !matches!(event, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown) {
        return false;
    }
    let Some(address) = address else {
        return false;
    };
    let now = Instant::now();
    let writing = state.grant(address, now).is_some();
    let active_provider = if writing {
        active_agent_provider(state, address)
    } else {
        None
    };
    let codex_candidate = active_provider.is_none_or(|kind| kind == PROVIDER_CODEX);
    let allow_open = state
        .codex_open_attempt(address)
        .is_none_or(|sent| now.duration_since(sent) >= CODEX_OPEN_COOLDOWN);
    let wheel = {
        let Some(surface) = state.surfaces.get(&address) else {
            return false;
        };
        let screen = surface.screen();
        if writing && screen.mouse_protocol_mode() != vt100::MouseProtocolMode::None {
            return false;
        }
        if writing
            && codex_candidate
            && let Some(wheel) = codex_wheel_keys(screen, allow_open, event)
        {
            Some(wheel)
        } else if writing && screen.alternate_screen() {
            // Alternate-screen programs own the visible transcript. With no
            // mouse protocol, terminal scrollback cannot move, so translate the
            // wheel into the conventional page keys.
            Some(CodexWheel {
                keys: page_keys(event),
                opened_transcript: false,
                transcript_visible: false,
            })
        } else {
            None
        }
    };
    if let Some(wheel) = wheel {
        if wheel.opened_transcript {
            state.set_codex_open_attempt(address, now);
        }
        if wheel.transcript_visible {
            state.clear_codex_open_attempt(address);
        }
        if !wheel.keys.is_empty() {
            let _ = crate::terminal_actions::deliver(state, jobs, Ok(wheel.keys));
        }
        return true;
    }
    let Some(surface) = state.surfaces.get_mut(&address) else {
        return false;
    };
    let previous = surface.screen().scrollback();
    let rows = if event == MouseEventKind::ScrollUp {
        previous.saturating_add(crate::constants::SCROLL_ROWS_PER_TICK)
    } else {
        previous.saturating_sub(crate::constants::SCROLL_ROWS_PER_TICK)
    };
    surface.set_scrollback(rows);
    true
}

fn page_keys(event: MouseEventKind) -> Vec<u8> {
    if event == MouseEventKind::ScrollUp {
        PAGE_UP.to_vec()
    } else {
        PAGE_DOWN.to_vec()
    }
}

fn active_agent_provider(state: &State, address: TerminalAddress) -> Option<&str> {
    let terminal = state.terminals.get(&address)?;
    let fields = terminal.fields();
    let agent = fields
        .agent
        .and_then(|target| state.inventory.agent_for_target(target))?;
    yolo_tui_lifecycle::active_agent(fields, Some(agent), state.inventory.current)
        .map(|agent| agent.fields().provider.kind())
}

fn codex_wheel_keys(
    screen: &vt100::Screen,
    allow_open: bool,
    event: MouseEventKind,
) -> Option<CodexWheel> {
    // A locally scrolled viewport can show an earlier Codex frame even though
    // the live foreground program is now the shell. Only the live tail may
    // generate provider keys; scrolled-back content stays local.
    if screen.scrollback() != 0 {
        return None;
    }
    let (rows, columns) = screen.size();
    let top = screen.contents_between(0, 0, 0, columns);
    let bottom = screen.contents_between(
        rows.saturating_sub(CODEX_COMPOSER_ROWS),
        0,
        rows.saturating_sub(1),
        columns,
    );
    if top.contains(CODEX_TRANSCRIPT_MARKER) || bottom.contains(CODEX_TRANSCRIPT_HINT) {
        return Some(CodexWheel {
            keys: codex_scroll_keys(event),
            opened_transcript: false,
            transcript_visible: true,
        });
    }
    // The visible composer is the authority for sending keys: agent
    // observation can lag a Codex that has exited or been suspended, while the
    // screen then belongs to the shell and Ctrl-T/arrows would edit the user's
    // command line. Codex renders its prompt directly above a model/cwd footer
    // and nothing follows that footer, so require the whole bottom block
    // instead of matching those fragments anywhere on screen.
    if !codex_composer_visible(&bottom) {
        return None;
    }
    if event == MouseEventKind::ScrollUp {
        let mut keys = Vec::new();
        if allow_open {
            keys.extend_from_slice(CODEX_OPEN_TRANSCRIPT);
        }
        keys.extend_from_slice(&codex_scroll_keys(event));
        return Some(CodexWheel {
            keys,
            opened_transcript: allow_open,
            transcript_visible: false,
        });
    }
    // Scrolling down at the live tail has nothing to reveal. If ordinary shell
    // history predates Codex, the caller can still move through it.
    (screen.scrollback() == 0).then(|| CodexWheel {
        keys: Vec::new(),
        opened_transcript: false,
        transcript_visible: false,
    })
}

include!("wheel_codex.rs");
