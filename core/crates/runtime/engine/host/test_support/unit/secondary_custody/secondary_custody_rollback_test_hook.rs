use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Mutex, OnceLock};
use tokio::sync::oneshot;
use uuid::Uuid;

const BOUNDARY_ALREADY_INSTALLED_DETAIL: &str =
    "pre-activation rollback boundary is already installed for this session";
const REACHED_RECEIVER_TAKEN_DETAIL: &str =
    "pre-activation rollback reached receiver was already taken";

struct Boundary {
    reached: Option<oneshot::Sender<()>>,
    resume: Option<oneshot::Receiver<()>>,
}

pub(in crate::host::secondary_custody) struct Guard {
    terminal_id: Uuid,
    reached: Option<oneshot::Receiver<()>>,
    resume: Option<oneshot::Sender<()>>,
}

pub(in crate::host::secondary_custody) fn install(terminal_id: Uuid) -> Guard {
    let (reached_sender, reached) = oneshot::channel();
    let (resume, resume_receiver) = oneshot::channel();
    match super::super::super::lock_recover(boundaries()).entry(terminal_id) {
        Entry::Vacant(slot) => {
            slot.insert(Boundary {
                reached: Some(reached_sender),
                resume: Some(resume_receiver),
            });
        }
        Entry::Occupied(_) => panic!("{BOUNDARY_ALREADY_INSTALLED_DETAIL}"),
    }
    Guard {
        terminal_id,
        reached: Some(reached),
        resume: Some(resume),
    }
}

pub(super) async fn pause_after_platform_drop(terminal_id: Uuid) {
    let claimed = {
        let mut boundaries = super::super::super::lock_recover(boundaries());
        boundaries
            .get_mut(&terminal_id)
            .and_then(|boundary| boundary.reached.take().zip(boundary.resume.take()))
    };
    let Some((reached, resume)) = claimed else {
        return;
    };
    let _ = reached.send(());
    let _ = resume.await;
}

impl Guard {
    pub(in crate::host::secondary_custody) fn take_reached(&mut self) -> oneshot::Receiver<()> {
        self.reached.take().expect(REACHED_RECEIVER_TAKEN_DETAIL)
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.resume.take();
        super::super::super::lock_recover(boundaries()).remove(&self.terminal_id);
    }
}

fn boundaries() -> &'static Mutex<HashMap<Uuid, Boundary>> {
    static BOUNDARIES: OnceLock<Mutex<HashMap<Uuid, Boundary>>> = OnceLock::new();
    BOUNDARIES.get_or_init(|| Mutex::new(HashMap::new()))
}
