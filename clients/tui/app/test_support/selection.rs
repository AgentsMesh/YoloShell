use super::panes;
use crate::{State, actions, network::Job, pointer};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use tokio::sync::mpsc;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_terminal_protocol::{OutputChunk, OutputChunkFields, TerminalEvent};
use yolo_tui_render::PaneViewport;

pub const TEXT: &[u8] = b"\x1b[?1002h\x1b[?1006halpha\r\nbeta";
pub const REPLACEMENT: &[u8] = b"\x1b[2J\x1b[Hchanged";
pub const EXPECTED_COPY: &[u8] = b"\x1b]52;c;YWxwaGE=\x1b\\";
pub const PROVIDER_COPY: &[u8] = b"\x1b]52;c;cHJvdmlkZXI=\x07";
pub const FIRST: u16 = 0;
pub const NEXT: u16 = 1;
pub const LAST_ALPHA: u16 = 4;

pub struct Fixture {
    pub state: State,
    pub target: RuntimeTarget,
    pub view: PaneViewport,
    pub jobs: mpsc::Sender<Job>,
    pub incoming: mpsc::Receiver<Job>,
}

impl Fixture {
    pub fn new() -> Self {
        let (mut state, _, second) = panes::state();
        panes::writable(&mut state, &second, TEXT);
        state.writers.clear();
        panes::repaint(&mut state);
        let target = second.fields().runtime.unwrap().target;
        let view = state
            .presentation
            .panes
            .iter()
            .find(|view| view.pane == state.tab_layout().focused())
            .copied()
            .unwrap();
        let (jobs, incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
        Self {
            state,
            target,
            view,
            jobs,
            incoming,
        }
    }

    pub fn begin(&mut self, column: u16, row: u16) {
        let mut event = panes::mouse(
            MouseEventKind::Down(MouseButton::Left),
            self.view.inner.x + column,
            self.view.inner.y + row,
        );
        event.modifiers = KeyModifiers::SHIFT;
        pointer::handle(&mut self.state, &self.jobs, event);
        panes::repaint(&mut self.state);
    }

    pub fn finish(&mut self, column: u16, row: u16) {
        pointer::handle(
            &mut self.state,
            &self.jobs,
            panes::mouse(
                MouseEventKind::Up(MouseButton::Left),
                self.view.inner.x + column,
                self.view.inner.y + row,
            ),
        );
    }

    pub fn select(&mut self) {
        self.begin(FIRST, FIRST);
        self.finish(LAST_ALPHA, FIRST);
    }

    pub fn copy(&mut self) {
        self.key(KeyCode::Char('b'), KeyModifiers::CONTROL);
        self.key(
            KeyCode::Char(crate::selection_state::COPY_KEY),
            KeyModifiers::NONE,
        );
    }

    pub fn key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        actions::handle(
            &mut self.state,
            &self.jobs,
            self.target.address.hub_id,
            Event::Key(KeyEvent::new(code, modifiers)),
        );
    }

    pub fn emit(&mut self, bytes: &[u8]) {
        let surface = self.state.surfaces.get_mut(&self.target.address).unwrap();
        let event = TerminalEvent::Output {
            chunk: OutputChunk::new(OutputChunkFields {
                target: self.target,
                sequence: surface.cursor().after_sequence + u64::from(NEXT),
                data: bytes.to_vec(),
            })
            .unwrap(),
        };
        surface.apply_event(&event).unwrap();
    }

    pub fn no_input(&mut self) {
        assert!(self.incoming.try_recv().is_err());
        assert!(self.state.writers.is_empty());
    }
}
