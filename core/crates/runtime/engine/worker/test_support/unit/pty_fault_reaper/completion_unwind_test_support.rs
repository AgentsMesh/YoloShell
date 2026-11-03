use super::FaultReap;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

type CompletionHook = Box<dyn FnOnce(&mut FaultReap)>;

const HOOK_ALREADY_INSTALLED: &str = "completion unwind hook is already installed";

thread_local! {
    static BEFORE_COMPLETION: RefCell<Option<CompletionHook>> = const { RefCell::new(None) };
}

pub(super) struct CompletionHookGuard(PhantomData<Rc<()>>);

pub(super) fn install(hook: impl FnOnce(&mut FaultReap) + 'static) -> CompletionHookGuard {
    BEFORE_COMPLETION.with(|slot| {
        let mut pending = slot.borrow_mut();
        assert!(pending.is_none(), "{HOOK_ALREADY_INSTALLED}");
        *pending = Some(Box::new(hook));
    });
    CompletionHookGuard(PhantomData)
}

pub(super) fn before_completion(ownership: &mut FaultReap) {
    let hook = BEFORE_COMPLETION.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook(ownership);
    }
}

impl Drop for CompletionHookGuard {
    fn drop(&mut self) {
        BEFORE_COMPLETION.with(|slot| {
            slot.borrow_mut().take();
        });
    }
}
