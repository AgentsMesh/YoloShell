use crate::checkpoint::Checkpoint;
use std::cell::RefCell;

type Hook = Box<dyn FnMut(Checkpoint) -> anyhow::Result<()>>;
thread_local! { static HOOK: RefCell<Option<Hook>> = RefCell::new(None); }

pub struct Guard;
pub fn install(hook: impl FnMut(Checkpoint) -> anyhow::Result<()> + 'static) -> Guard {
    HOOK.with(|value| {
        assert!(value.borrow_mut().replace(Box::new(hook)).is_none());
    });
    Guard
}
pub fn reach(point: Checkpoint) -> anyhow::Result<()> {
    HOOK.with(|value| match value.borrow_mut().as_mut() {
        Some(hook) => hook(point),
        None => Ok(()),
    })
}
impl Drop for Guard {
    fn drop(&mut self) {
        HOOK.with(|value| {
            value.borrow_mut().take();
        });
    }
}
