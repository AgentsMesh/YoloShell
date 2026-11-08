use std::cell::RefCell;

const HOOK_ALREADY_INSTALLED: &str = "namespace cleanup hook is already installed";
type Hook = Box<dyn FnOnce()>;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::paths::socket_namespace) enum Point {
    AfterRegisteredStageChunk,
    AfterEndpointInventory,
    AfterOrphanStageChunk,
}

thread_local! {
    static PENDING: RefCell<Option<(Point, Hook)>> = const { RefCell::new(None) };
}

pub(super) struct HookGuard;

pub(super) fn install(point: Point, hook: impl FnOnce() + 'static) -> HookGuard {
    PENDING.with(|slot| {
        let previous = slot.borrow_mut().replace((point, Box::new(hook)));
        assert!(previous.is_none(), "{HOOK_ALREADY_INSTALLED}");
    });
    HookGuard
}

pub(in crate::paths::socket_namespace) fn run(point: Point) {
    let hook = PENDING.with(|slot| {
        let mut pending = slot.borrow_mut();
        if pending
            .as_ref()
            .is_some_and(|(expected, _)| *expected == point)
        {
            pending.take().map(|(_, hook)| hook)
        } else {
            None
        }
    });
    if let Some(hook) = hook {
        hook();
    }
}

impl Drop for HookGuard {
    fn drop(&mut self) {
        PENDING.with(|slot| *slot.borrow_mut() = None);
    }
}
