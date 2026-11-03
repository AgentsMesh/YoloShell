use anyhow::Result;

type PersistenceHook = Box<dyn FnOnce() -> Result<()>>;

thread_local! {
    static BEFORE_WRITE: std::cell::RefCell<Option<PersistenceHook>> =
        const { std::cell::RefCell::new(None) };
}

pub(super) fn install(hook: impl FnOnce() -> Result<()> + 'static) {
    BEFORE_WRITE.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

pub(super) fn run() -> Result<()> {
    BEFORE_WRITE.with(|slot| match slot.borrow_mut().take() {
        Some(hook) => hook(),
        None => Ok(()),
    })
}
