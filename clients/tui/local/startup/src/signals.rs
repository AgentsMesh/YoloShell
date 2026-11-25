use anyhow::Result;
use tokio::signal::unix::{Signal, SignalKind, signal};

pub(crate) struct Cancellation {
    interrupt: Signal,
    terminate: Signal,
    hangup: Signal,
    quit: Signal,
}
impl Cancellation {
    pub fn install() -> Result<Self> {
        Ok(Self {
            interrupt: signal(SignalKind::interrupt())?,
            terminate: signal(SignalKind::terminate())?,
            hangup: signal(SignalKind::hangup())?,
            quit: signal(SignalKind::quit())?,
        })
    }
    pub async fn cancelled(&mut self) {
        tokio::select! {
            _ = self.interrupt.recv() => {},
            _ = self.terminate.recv() => {},
            _ = self.hangup.recv() => {},
            _ = self.quit.recv() => {},
        }
    }
}
