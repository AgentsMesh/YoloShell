use crate::fake_agent_constants::*;
use std::error::Error;
use std::io::{BufRead, Write};

pub(crate) fn run() -> Result<(), Box<dyn Error>> {
    if let Some(marker) = launch_marker(marker_environment())? {
        println!("{marker}");
        std::io::stdout().flush()?;
    }
    for line in std::io::stdin().lock().lines() {
        if let Some(marker) = ping_marker(&line?) {
            println!("{marker}");
            std::io::stdout().flush()?;
        }
    }
    if hold_after_eof(std::env::var_os(HOLD_AFTER_EOF_ENV).as_deref()) {
        loop {
            std::thread::park();
        }
    }
    Ok(())
}

pub(crate) fn hold_after_eof(value: Option<&std::ffi::OsStr>) -> bool {
    value == Some(std::ffi::OsStr::new(HOLD_AFTER_EOF_ENABLED))
}

fn marker_environment() -> [Option<String>; 3] {
    [
        std::env::var(PACKAGED_MARKER_LEFT_ENV).ok(),
        std::env::var(PACKAGED_MARKER_RIGHT_ENV).ok(),
        std::env::var(PACKAGED_MARKER_NONCE_ENV).ok(),
    ]
}

fn launch_marker(values: [Option<String>; 3]) -> Result<Option<String>, &'static str> {
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    let [Some(left), Some(right), Some(nonce)] = values else {
        return Err(CODEX_TUI_MARKER_ERROR);
    };
    Ok(Some(format!("{left}{right}{nonce}")))
}

fn ping_marker(line: &str) -> Option<String> {
    let fields = line.split_whitespace().collect::<Vec<_>>();
    match fields.as_slice() {
        [command, left, right, nonce] if *command == PACKAGED_TUI_PING_COMMAND => {
            Some(format!("{left}{right}{nonce}"))
        }
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn launch_marker_for_test(
    values: [Option<String>; 3],
) -> Result<Option<String>, &'static str> {
    launch_marker(values)
}

#[cfg(test)]
pub(crate) fn ping_marker_for_test(line: &str) -> Option<String> {
    ping_marker(line)
}
