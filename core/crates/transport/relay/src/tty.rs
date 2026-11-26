use super::TerminalSize;
use std::io;
use std::os::fd::RawFd;
use tokio::sync::mpsc;

const RESIZE_CHANNEL_CAPACITY: usize = 8;

pub struct RawTerminal {
    fd: RawFd,
    original: libc::termios,
}

impl RawTerminal {
    pub fn enable(fd: RawFd) -> io::Result<Option<Self>> {
        if !is_terminal(fd)? {
            return Ok(None);
        }
        let mut original = std::mem::MaybeUninit::<libc::termios>::uninit();
        // SAFETY: fd is open and tcgetattr initializes the supplied termios on success.
        if unsafe { libc::tcgetattr(fd, original.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: tcgetattr succeeded, so original is initialized.
        let original = unsafe { original.assume_init() };
        let mut raw = original;
        // SAFETY: raw is an initialized termios structure.
        unsafe { libc::cfmakeraw(&mut raw) };
        // SAFETY: fd is open and raw points to a valid termios structure.
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Some(Self { fd, original }))
    }
}

impl Drop for RawTerminal {
    fn drop(&mut self) {
        // SAFETY: original came from this fd. Restoration is best-effort during drop.
        let _ = unsafe { libc::tcsetattr(self.fd, libc::TCSANOW, &self.original) };
    }
}

pub fn terminal_size(fd: RawFd) -> io::Result<Option<TerminalSize>> {
    if !is_terminal(fd)? {
        return Ok(None);
    }
    let mut size = std::mem::MaybeUninit::<libc::winsize>::zeroed();
    // SAFETY: fd is a terminal and ioctl initializes the supplied winsize on success.
    if unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, size.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: ioctl succeeded, so size is initialized.
    let size = unsafe { size.assume_init() };
    Ok(TerminalSize::new(size.ws_col, size.ws_row))
}

pub fn watch_window_changes(fd: RawFd) -> io::Result<mpsc::Receiver<TerminalSize>> {
    let mut signals =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::window_change())?;
    let (sender, receiver) = mpsc::channel(RESIZE_CHANNEL_CAPACITY);
    tokio::spawn(async move {
        while signals.recv().await.is_some() {
            if !publish_current_size(fd, &sender).await {
                break;
            }
        }
    });
    Ok(receiver)
}

async fn publish_current_size(fd: RawFd, sender: &mpsc::Sender<TerminalSize>) -> bool {
    let Ok(Some(size)) = terminal_size(fd) else {
        return true;
    };
    sender.send(size).await.is_ok()
}

fn is_terminal(fd: RawFd) -> io::Result<bool> {
    // SAFETY: isatty only inspects the supplied file descriptor.
    if unsafe { libc::isatty(fd) } == 1 {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ENOTTY) {
        Ok(false)
    } else {
        Err(error)
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tty_publish_tests.rs"]
mod publish_tests;
