mod claude;
pub(crate) mod codex;
pub mod constants;
mod parser;
use crate::{packaged_claude_settings::Settings, packaged_claude_transcript::Transcript};
use constants::*;
use std::{
    error::Error,
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
};
#[cfg(test)]
#[path = "../unit_tests/chat_io.rs"]
mod tests;

pub(crate) fn configured() -> bool {
    std::env::var(ENABLE_ENV).as_deref() == Ok(ENABLED)
}

pub(crate) fn run(
    settings: &Settings,
    transcript: &mut Transcript,
    session: &str,
) -> Result<(), Box<dyn Error>> {
    run_editor(|text| {
        let cwd = std::env::current_dir()?.canonicalize()?;
        claude::prompt(settings, session, transcript.path(), &cwd)?;
        transcript.chat_message(session, &cwd, USER, text)?;
        transcript.chat_message(session, &cwd, ASSISTANT, ANSWER)?;
        claude::ready(settings, session, transcript.path(), &cwd)
    })
}

fn run_editor(
    mut submitted: impl FnMut(&str) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let marker = PathBuf::from(std::env::var_os(MARKER_ENV).ok_or(MARKER_INVALID)?);
    if !marker.is_absolute() {
        return Err(MARKER_INVALID.into());
    }
    let mut marker = OpenOptions::new()
        .append(true)
        .create_new(true)
        .mode(PRIVATE_MODE)
        .open(marker)?;
    let _terminal = RawTerminal::open()?;
    std::io::stdout().write_all(ENABLE_PASTE)?;
    println!("{READY}\r");
    std::io::stdout().flush()?;
    let mut editor = parser::Editor::default();
    let mut input = [0; READ_BYTES];
    loop {
        let size = std::io::stdin().read(&mut input)?;
        if size == 0 {
            break;
        }
        for bytes in editor.feed(&input[..size])? {
            let text = std::str::from_utf8(&bytes)?;
            serde_json::to_writer(&mut marker, &text)?;
            marker.write_all(b"\n")?;
            marker.flush()?;
            marker.sync_data()?;
            submitted(text)?;
            println!("{SENT}{text}\r");
        }
        if let Ok(draft) = std::str::from_utf8(editor.draft()) {
            println!("{DRAFT}{draft}\r");
        }
        std::io::stdout().flush()?;
    }
    Ok(())
}

struct RawTerminal(libc::termios);
impl RawTerminal {
    fn open() -> std::io::Result<Self> {
        let mut original: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut original) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        let mut raw = original;
        unsafe { libc::cfmakeraw(&mut raw) };
        if unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self(original))
    }
}
impl Drop for RawTerminal {
    fn drop(&mut self) {
        let _ = std::io::stdout().write_all(DISABLE_PASTE);
        let _ = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.0) };
    }
}
