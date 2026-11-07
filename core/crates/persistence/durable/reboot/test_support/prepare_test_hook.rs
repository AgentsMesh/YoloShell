use anyhow::Result;
use std::cell::RefCell;

const HOOK_OCCUPIED: &str = "durable reboot prepare test hook is occupied";
type Hook = Box<dyn FnMut() -> Result<()>>;

thread_local! {
    static AFTER_AUTHORITY_PUBLISH: RefCell<Option<Hook>> = RefCell::new(None);
    static AFTER_ORPHAN_PARTICIPANT: RefCell<Option<Hook>> = RefCell::new(None);
    static AFTER_STAGE: RefCell<Option<Hook>> = RefCell::new(None);
    static AFTER_INTENT_COMMIT: RefCell<Option<Hook>> = RefCell::new(None);
    static BEFORE_INTENT_COMMIT: RefCell<Option<Hook>> = RefCell::new(None);
    static BEFORE_INTENT_CLEAR: RefCell<Option<Hook>> = RefCell::new(None);
    static BEFORE_ORPHAN_MUTATION: RefCell<Option<Hook>> = RefCell::new(None);
    static BEFORE_PREPARATION_RETIRE: RefCell<Option<Hook>> = RefCell::new(None);
    static BEFORE_SETTLED_PUBLISH: RefCell<Option<Hook>> = RefCell::new(None);
}

pub(super) struct HookGuard;

macro_rules! hook_pair {
    ($install:ident, $run:ident, $slot:ident) => {
        pub(super) fn $install(hook: impl FnMut() -> Result<()> + 'static) -> HookGuard {
            $slot.with(|slot| {
                let previous = slot.borrow_mut().replace(Box::new(hook));
                assert!(previous.is_none(), "{HOOK_OCCUPIED}");
            });
            HookGuard
        }

        pub(super) fn $run() -> Result<()> {
            match $slot.with(|slot| slot.borrow_mut().as_mut().map(|hook| hook())) {
                Some(result) => result,
                None => Ok(()),
            }
        }
    };
}

hook_pair!(
    install_after_authority_publish,
    run_after_authority_publish,
    AFTER_AUTHORITY_PUBLISH
);
hook_pair!(
    install_before_intent_clear,
    run_before_intent_clear,
    BEFORE_INTENT_CLEAR
);
hook_pair!(
    install_before_settled_publish,
    run_before_settled_publish,
    BEFORE_SETTLED_PUBLISH
);
hook_pair!(
    install_after_orphan_participant,
    run_after_orphan_participant,
    AFTER_ORPHAN_PARTICIPANT
);
hook_pair!(
    install_before_orphan_mutation,
    run_before_orphan_mutation,
    BEFORE_ORPHAN_MUTATION
);
hook_pair!(
    install_before_preparation_retire,
    run_before_preparation_retire,
    BEFORE_PREPARATION_RETIRE
);

pub(super) fn install_after_stage(hook: impl FnMut() -> Result<()> + 'static) -> HookGuard {
    AFTER_STAGE.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "{HOOK_OCCUPIED}");
    });
    HookGuard
}

pub(super) fn run_after_stage() -> Result<()> {
    match AFTER_STAGE.with(|slot| slot.borrow_mut().as_mut().map(|hook| hook())) {
        Some(result) => result,
        None => Ok(()),
    }
}

pub(super) fn install_after_intent_commit(hook: impl FnMut() -> Result<()> + 'static) -> HookGuard {
    AFTER_INTENT_COMMIT.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "{HOOK_OCCUPIED}");
    });
    HookGuard
}

pub(super) fn run_after_intent_commit() -> Result<()> {
    match AFTER_INTENT_COMMIT.with(|slot| slot.borrow_mut().as_mut().map(|hook| hook())) {
        Some(result) => result,
        None => Ok(()),
    }
}

pub(super) fn install_before_intent_commit(
    hook: impl FnMut() -> Result<()> + 'static,
) -> HookGuard {
    BEFORE_INTENT_COMMIT.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "{HOOK_OCCUPIED}");
    });
    HookGuard
}

pub(super) fn run_before_intent_commit() -> Result<()> {
    match BEFORE_INTENT_COMMIT.with(|slot| slot.borrow_mut().as_mut().map(|hook| hook())) {
        Some(result) => result,
        None => Ok(()),
    }
}

impl Drop for HookGuard {
    fn drop(&mut self) {
        AFTER_AUTHORITY_PUBLISH.with(|slot| *slot.borrow_mut() = None);
        AFTER_ORPHAN_PARTICIPANT.with(|slot| *slot.borrow_mut() = None);
        AFTER_STAGE.with(|slot| *slot.borrow_mut() = None);
        AFTER_INTENT_COMMIT.with(|slot| *slot.borrow_mut() = None);
        BEFORE_INTENT_COMMIT.with(|slot| *slot.borrow_mut() = None);
        BEFORE_INTENT_CLEAR.with(|slot| *slot.borrow_mut() = None);
        BEFORE_ORPHAN_MUTATION.with(|slot| *slot.borrow_mut() = None);
        BEFORE_PREPARATION_RETIRE.with(|slot| *slot.borrow_mut() = None);
        BEFORE_SETTLED_PUBLISH.with(|slot| *slot.borrow_mut() = None);
    }
}
