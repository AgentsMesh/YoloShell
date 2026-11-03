use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::process::ExitStatus;

const NESTED_SCRIPT_DETAIL: &str = "private input settlement script already installed";

#[derive(Clone, Copy, Debug)]
pub(super) enum PollOverride {
    Running,
    Interrupted,
    Failure(&'static str),
}

#[derive(Default)]
pub(super) struct SettlementScript {
    polls: VecDeque<PollOverride>,
    termination_failure: Option<&'static str>,
}

impl SettlementScript {
    pub(super) fn new(polls: impl IntoIterator<Item = PollOverride>) -> Self {
        Self {
            polls: polls.into_iter().collect(),
            ..Self::default()
        }
    }

    pub(super) fn fail_termination(mut self, detail: &'static str) -> Self {
        self.termination_failure = Some(detail);
        self
    }
}

thread_local! {
    static SCRIPT: RefCell<Option<SettlementScript>> = const { RefCell::new(None) };
}

pub(super) struct SettlementScriptGuard;

impl Drop for SettlementScriptGuard {
    fn drop(&mut self) {
        SCRIPT.with_borrow_mut(|slot| {
            slot.take();
        });
    }
}

pub(super) fn install(script: SettlementScript) -> SettlementScriptGuard {
    SCRIPT.with_borrow_mut(|slot| {
        assert!(slot.replace(script).is_none(), "{NESTED_SCRIPT_DETAIL}");
    });
    SettlementScriptGuard
}

pub(super) fn poll_override() -> Option<io::Result<Option<ExitStatus>>> {
    SCRIPT.with_borrow_mut(|slot| {
        slot.as_mut()?.polls.pop_front().map(|action| match action {
            PollOverride::Running => Ok(None),
            PollOverride::Interrupted => Err(io::ErrorKind::Interrupted.into()),
            PollOverride::Failure(detail) => Err(io::Error::other(detail)),
        })
    })
}

pub(super) fn termination_failure() -> Option<io::Error> {
    take_failure(|script| &mut script.termination_failure)
}

fn take_failure(
    select: impl FnOnce(&mut SettlementScript) -> &mut Option<&'static str>,
) -> Option<io::Error> {
    SCRIPT.with_borrow_mut(|slot| select(slot.as_mut()?).take().map(io::Error::other))
}
