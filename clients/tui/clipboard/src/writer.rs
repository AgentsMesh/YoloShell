use std::{
    future::{Future, pending},
    pin::Pin,
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};
use yolo_tui_selection::ClipboardCommand;

const DESKTOP_TIMEOUT: Duration = Duration::from_secs(2);
const OUTER_TERMINAL: &[&str] = &[
    "SSH_CONNECTION",
    "SSH_TTY",
    "TMUX",
    "STY",
    "YOLO_TERMINAL_ID",
];

type Write = Pin<Box<dyn Future<Output = Option<ClipboardCommand>> + Send>>;

/// A desktop client writes the local clipboard. Headless and nested clients
/// send OSC 52 to the terminal displaying the user's selection.
#[derive(Default)]
pub struct Writer {
    writing: Option<Write>,
    queued: Option<ClipboardCommand>,
}

impl Writer {
    pub fn queue(&mut self, selection: ClipboardCommand) -> Option<ClipboardCommand> {
        if OUTER_TERMINAL
            .iter()
            .any(|name| std::env::var_os(name).is_some())
            || !desktop()
        {
            return Some(selection);
        }
        self.queued = Some(selection);
        None
    }

    pub async fn fallback(&mut self) -> Option<ClipboardCommand> {
        if self.writing.is_none() {
            let Some(selection) = self.queued.take() else {
                return pending().await;
            };
            self.writing = Some(Box::pin(async move {
                let wrote = timeout(DESKTOP_TIMEOUT, write(selection.text().as_bytes()))
                    .await
                    .unwrap_or(false);
                (!wrote).then_some(selection)
            }));
        }
        let result = self.writing.as_mut()?.await;
        self.writing = None;
        result
    }
}

#[cfg(target_os = "macos")]
fn desktop() -> bool {
    true
}

#[cfg(target_os = "macos")]
async fn write(bytes: &[u8]) -> bool {
    send("/usr/bin/pbcopy", &[], bytes).await
}

#[cfg(target_os = "linux")]
fn desktop() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some()
}

#[cfg(target_os = "linux")]
async fn write(bytes: &[u8]) -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        && send(
            "wl-copy",
            &[crate::platform::MIME_TYPE, "text/plain;charset=utf-8"],
            bytes,
        )
        .await
    {
        return true;
    }
    std::env::var_os("DISPLAY").is_some()
        && send(
            "xclip",
            &["-selection", "clipboard", "-in", "-target", "UTF8_STRING"],
            bytes,
        )
        .await
}

async fn send(program: &str, args: &[&str], bytes: &[u8]) -> bool {
    let Ok(mut child) = Command::new(program)
        .args(args)
        .env("LC_CTYPE", "UTF-8")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
    else {
        return false;
    };
    let Some(mut stdin) = child.stdin.take() else {
        return false;
    };
    if stdin.write_all(bytes).await.is_err() {
        return false;
    }
    drop(stdin);
    child.wait().await.is_ok_and(|status| status.success())
}
