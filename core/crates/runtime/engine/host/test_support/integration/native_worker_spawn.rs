use std::cell::{Cell, RefCell};
use std::io;
use std::os::fd::RawFd;
use std::rc::Rc;

type Descriptors = [RawFd; super::HOST_INSTANCE_LOCK_COUNT];
type Hook = Box<dyn FnOnce(Descriptors) -> io::Result<()>>;

thread_local! {
    static SPAWN_HOOK: RefCell<Option<Hook>> = RefCell::new(None);
}

pub(super) struct SpawnHook {
    called: Rc<Cell<bool>>,
}

impl SpawnHook {
    pub(super) fn assert_called(&self) {
        assert!(
            self.called.get(),
            "native worker spawn fault was not exercised"
        );
    }
}

impl Drop for SpawnHook {
    fn drop(&mut self) {
        SPAWN_HOOK.with(|slot| slot.borrow_mut().take());
    }
}

pub(super) fn install(hook: impl FnOnce(Descriptors) -> io::Result<()> + 'static) -> SpawnHook {
    let called = Rc::new(Cell::new(false));
    let observed = Rc::clone(&called);
    SPAWN_HOOK.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(move |descriptors| {
            observed.set(true);
            hook(descriptors)
        }));
        assert!(
            previous.is_none(),
            "native worker spawn fault is already installed"
        );
    });
    SpawnHook { called }
}

pub(super) fn run(descriptors: Descriptors) -> io::Result<()> {
    let hook = SPAWN_HOOK.with(|slot| slot.borrow_mut().take());
    match hook {
        Some(hook) => hook(descriptors),
        None => Ok(()),
    }
}
