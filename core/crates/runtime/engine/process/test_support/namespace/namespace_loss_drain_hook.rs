use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;

const HOOK_ALREADY_INSTALLED_DETAIL: &str =
    "namespace-loss drain pending test hook is already installed";

thread_local! {
    static PENDING_ONCE: Cell<bool> = const { Cell::new(false) };
}

pub struct Guard(PhantomData<Rc<()>>);

pub fn install_pending_once() -> Guard {
    PENDING_ONCE.with(|pending| {
        assert!(!pending.replace(true), "{HOOK_ALREADY_INSTALLED_DETAIL}");
    });
    Guard(PhantomData)
}

pub(super) fn take_pending() -> bool {
    PENDING_ONCE.with(|pending| pending.replace(false))
}

impl Drop for Guard {
    fn drop(&mut self) {
        PENDING_ONCE.with(|pending| pending.set(false));
    }
}
