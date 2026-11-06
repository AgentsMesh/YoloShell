//! Terminal mode ownership for the interactive client: alternate screen,
//! bracketed paste, mouse and focus capture, and their restoration on drop.

use crossterm::{
    cursor::Show,
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, IsTerminal, Stdout, Write};

pub const INTERACTIVE_TERMINAL_REQUIRED: &str = "TUI requires an interactive terminal";

pub struct ScreenGuard {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl ScreenGuard {
    pub fn notify(&mut self, bell: &[u8]) -> io::Result<()> {
        self.terminal.backend_mut().write_all(bell)?;
        self.terminal.backend_mut().flush()
    }
    pub fn copy(&mut self, command: &yolo_tui_selection::ClipboardCommand) -> io::Result<()> {
        self.terminal.backend_mut().write_all(command.as_bytes())?;
        self.terminal.backend_mut().flush()
    }
    pub fn enter() -> io::Result<Self> {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                INTERACTIVE_TERMINAL_REQUIRED,
            ));
        }
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(
            stdout,
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableMouseCapture,
            EnableFocusChange
        ) {
            restore();
            return Err(error);
        }
        match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => Ok(Self { terminal }),
            Err(error) => {
                restore();
                Err(error)
            }
        }
    }
}

fn restore() {
    let _ = execute!(
        io::stdout(),
        DisableBracketedPaste,
        DisableMouseCapture,
        DisableFocusChange,
        LeaveAlternateScreen,
        Show
    );
    let _ = disable_raw_mode();
}

impl Drop for ScreenGuard {
    fn drop(&mut self) {
        restore();
    }
}
