use crate::process::LiveProcessIdentity;
use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;

const POST_INVENTORY_FORK_ALREADY_INSTALLED: &str = "post-inventory fork hook is already installed";
pub(super) const POST_OBSERVATION_FAILURE_DETAIL: &str =
    "injected failure after capability observation";

thread_local! {
    static POST_INVENTORY_FORK: Cell<Option<LiveProcessIdentity>> = const { Cell::new(None) };
    static FAIL_AFTER_OBSERVATION: Cell<bool> = const { Cell::new(false) };
}

pub(in crate::process) struct PostInventoryForkGuard(PhantomData<Rc<()>>);
pub(super) struct ObservationFailureGuard(PhantomData<Rc<()>>);

pub(super) fn install_post_inventory_fork(parent: LiveProcessIdentity) -> PostInventoryForkGuard {
    POST_INVENTORY_FORK.with(|slot| {
        assert!(
            slot.replace(Some(parent)).is_none(),
            "{POST_INVENTORY_FORK_ALREADY_INSTALLED}"
        );
    });
    PostInventoryForkGuard(PhantomData)
}

pub(super) fn take_post_inventory_fork() -> Option<LiveProcessIdentity> {
    POST_INVENTORY_FORK.with(|slot| slot.take())
}

pub(super) fn fail_after_next_observation() -> ObservationFailureGuard {
    FAIL_AFTER_OBSERVATION.with(|failure| assert!(!failure.replace(true)));
    ObservationFailureGuard(PhantomData)
}

pub(super) fn take_observation_failure() -> bool {
    FAIL_AFTER_OBSERVATION.with(|failure| failure.replace(false))
}

impl Drop for PostInventoryForkGuard {
    fn drop(&mut self) {
        POST_INVENTORY_FORK.with(|slot| slot.set(None));
    }
}

impl Drop for ObservationFailureGuard {
    fn drop(&mut self) {
        FAIL_AFTER_OBSERVATION.with(|failure| failure.set(false));
    }
}
