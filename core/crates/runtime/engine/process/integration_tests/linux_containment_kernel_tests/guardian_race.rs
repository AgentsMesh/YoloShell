use super::devpts_fixture::{MAX_REUSE_ALLOCATIONS, PtyPair};
use super::support::{KERNEL_POLL_INTERVAL, KERNEL_SETTLE_TIMEOUT};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Instant;

const INITIAL_RACE_ATTEMPTS: usize = 1;

pub struct ReuseRace {
    task: JoinHandle<ReuseResult>,
}

pub struct ReuseResult {
    pub reused: PtyPair,
    pub retained: Vec<PtyPair>,
    pub attempts: usize,
}

impl ReuseRace {
    pub fn start(path: &Path) -> Self {
        let path = path.to_path_buf();
        let (ready_sender, ready_receiver) = mpsc::sync_channel(0);
        let task = std::thread::spawn(move || allocate(path, ready_sender));
        ready_receiver
            .recv()
            .expect("allocator must attempt reuse while guardian holds the PTY");
        Self { task }
    }

    pub fn finish(self) -> ReuseResult {
        self.task.join().expect("devpts reuse allocator panicked")
    }
}

fn allocate(path: PathBuf, ready: mpsc::SyncSender<()>) -> ReuseResult {
    let deadline = Instant::now() + KERNEL_SETTLE_TIMEOUT;
    let first = PtyPair::open();
    assert_ne!(
        first.path, path,
        "guardian-held master alias allowed premature devpts reuse"
    );
    let mut retained = vec![first];
    let mut attempts = INITIAL_RACE_ATTEMPTS;
    ready.send(()).expect("publish first blocked allocation");
    loop {
        let candidate = PtyPair::open();
        attempts += 1;
        if candidate.path == path {
            return ReuseResult {
                reused: candidate,
                retained,
                attempts,
            };
        }
        retained.push(candidate);
        if retained.len() == MAX_REUSE_ALLOCATIONS {
            retained.clear();
            std::thread::sleep(KERNEL_POLL_INTERVAL);
        }
        assert!(
            Instant::now() < deadline,
            "guardian did not release devpts object"
        );
    }
}
