use super::worker::{ReapChild, advance};
use super::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

const EXPECTED_CHILD_SPAWNS: usize = 0;
const REAPER_LAUNCH_FAILURE: &str = "injected termination reaper launch failure";
const FIRST_CHILD_ID: u32 = 1;
const SECOND_CHILD_ID: u32 = 2;
const FIRST_POLL_FAILURE: &str = "injected first child poll failure";
const FIRST_KILL_FAILURE: &str = "injected first child kill failure";

#[test]
fn reaper_initialization_failure_prevents_any_child_spawn() {
    let child_spawns = AtomicUsize::new(usize::default());
    let result = TerminationReaper::start_with(|_| Err(io::Error::other(REAPER_LAUNCH_FAILURE)));
    if result.is_ok() {
        child_spawns.fetch_add(1, Ordering::SeqCst);
    }

    let error = result.err().unwrap();
    assert_eq!(error.to_string(), REAPER_LAUNCH_FAILURE);
    assert_eq!(child_spawns.load(Ordering::SeqCst), EXPECTED_CHILD_SPAWNS);
}

#[test]
fn persistent_first_child_failure_does_not_starve_later_termination() {
    let mut children = vec![
        retained(
            FIRST_CHILD_ID,
            Err(io::Error::other(FIRST_POLL_FAILURE)),
            Err(io::Error::other(FIRST_KILL_FAILURE)),
        ),
        retained(SECOND_CHILD_ID, Ok(false), Ok(())),
    ];

    let progress = advance(&mut children);

    assert_eq!(progress.settled, 0);
    assert_eq!(progress.terminations, 1);
    assert!(
        children
            .iter()
            .find(|child| child.child.id == FIRST_CHILD_ID)
            .unwrap()
            .termination_required
    );
    assert!(
        !children
            .iter()
            .find(|child| child.child.id == SECOND_CHILD_ID)
            .unwrap()
            .termination_required
    );
}

struct TestChild {
    id: u32,
    polls: VecDeque<io::Result<bool>>,
    terminations: VecDeque<io::Result<()>>,
}

impl ReapChild for TestChild {
    fn process_id(&self) -> u32 {
        self.id
    }

    fn try_reap(&mut self) -> io::Result<bool> {
        self.polls.pop_front().unwrap()
    }

    fn terminate(&mut self) -> io::Result<()> {
        self.terminations.pop_front().unwrap()
    }
}

fn retained(
    id: u32,
    poll: io::Result<bool>,
    termination: io::Result<()>,
) -> RetainedChild<TestChild> {
    RetainedChild {
        child: TestChild {
            id,
            polls: VecDeque::from([poll]),
            terminations: VecDeque::from([termination]),
        },
        termination_required: true,
    }
}
