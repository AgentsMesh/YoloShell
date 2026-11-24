use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;

const HOOK_ALREADY_INSTALLED_DETAIL: &str =
    "resilience kill commit handoff panic is already installed";
const HANDOFF_PANIC_DETAIL: &str = "injected resilience kill commit handoff panic";

thread_local! {
    static PANIC_AT_HANDOFF: Cell<bool> = const { Cell::new(false) };
}

pub(crate) struct TestKillCommitHandoffPanicGuard(PhantomData<Rc<()>>);

pub(crate) fn install_test_kill_commit_handoff_panic() -> TestKillCommitHandoffPanicGuard {
    PANIC_AT_HANDOFF.with(|installed| {
        assert!(!installed.replace(true), "{HOOK_ALREADY_INSTALLED_DETAIL}");
    });
    TestKillCommitHandoffPanicGuard(PhantomData)
}

pub(super) fn panic_at_handoff() {
    PANIC_AT_HANDOFF.with(|installed| {
        if installed.replace(false) {
            panic!("{HANDOFF_PANIC_DETAIL}");
        }
    });
}

impl Drop for TestKillCommitHandoffPanicGuard {
    fn drop(&mut self) {
        PANIC_AT_HANDOFF.with(|installed| installed.set(false));
    }
}
