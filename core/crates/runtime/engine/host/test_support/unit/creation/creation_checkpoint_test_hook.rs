use anyhow::Result;
use std::cell::RefCell;

const HOOK_ALREADY_INSTALLED: &str = "creation checkpoint test hook is already installed";

type Hook = Box<dyn FnMut() -> Result<()>>;

thread_local! {
    static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) };
}

pub(crate) struct Guard(std::marker::PhantomData<std::rc::Rc<()>>);

pub(crate) fn install(hook: impl FnMut() -> Result<()> + 'static) -> Guard {
    HOOK.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "{HOOK_ALREADY_INSTALLED}");
    });
    Guard(std::marker::PhantomData)
}

pub(crate) fn run() -> Result<()> {
    HOOK.with(|slot| match slot.borrow_mut().as_mut() {
        Some(hook) => hook(),
        None => Ok(()),
    })
}

impl Drop for Guard {
    fn drop(&mut self) {
        HOOK.with(|slot| *slot.borrow_mut() = None);
    }
}
