use super::*;
use crate::host::lock_recover;
use std::sync::{Mutex, OnceLock};
use tokio::sync::oneshot;

struct Barrier {
    terminal_id: Uuid,
    reached: oneshot::Sender<()>,
    resume: oneshot::Receiver<()>,
}

pub(in crate::host::secondary_custody) struct Guard {
    terminal_id: Uuid,
    reached: Option<oneshot::Receiver<()>>,
    resume: Option<oneshot::Sender<()>>,
}

pub(in crate::host::secondary_custody) fn install(terminal_id: Uuid) -> Guard {
    let (reached_sender, reached) = oneshot::channel();
    let (resume, resume_receiver) = oneshot::channel();
    let mut barriers = lock_recover(barriers());
    assert!(
        !barriers
            .iter()
            .any(|barrier| barrier.terminal_id == terminal_id)
    );
    barriers.push(Barrier {
        terminal_id,
        reached: reached_sender,
        resume: resume_receiver,
    });
    Guard {
        terminal_id,
        reached: Some(reached),
        resume: Some(resume),
    }
}

pub(super) async fn pause_after_fault_transition(terminal_id: Uuid) {
    let barrier = {
        let mut barriers = lock_recover(barriers());
        barriers
            .iter()
            .position(|barrier| barrier.terminal_id == terminal_id)
            .map(|index| barriers.swap_remove(index))
    };
    if let Some(barrier) = barrier {
        let _ = barrier.reached.send(());
        let _ = barrier.resume.await;
    }
}

impl Guard {
    pub(in crate::host::secondary_custody) async fn reached(&mut self, host: &Host) {
        tokio::time::timeout(host.worker_ready_timeout, self.reached.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    }

    pub(in crate::host::secondary_custody) fn resume(mut self) {
        self.resume.take().unwrap().send(()).unwrap();
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let mut barriers = lock_recover(barriers());
        barriers.retain(|barrier| barrier.terminal_id != self.terminal_id);
    }
}

fn barriers() -> &'static Mutex<Vec<Barrier>> {
    static BARRIERS: OnceLock<Mutex<Vec<Barrier>>> = OnceLock::new();
    BARRIERS.get_or_init(Mutex::default)
}
