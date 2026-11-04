use anyhow::Result;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Checkpoint {
    BeforeCommit,
    AfterCommit,
}
pub(crate) fn reach(point: Checkpoint) -> Result<()> {
    #[cfg(test)]
    return tests::reach(point);
    #[cfg(not(test))]
    {
        let _ = point;
        Ok(())
    }
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::cell::RefCell;
    type Hook = Box<dyn FnMut(Checkpoint) -> Result<()>>;
    thread_local! { static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) }; }
    pub struct Guard;
    pub fn install(hook: impl FnMut(Checkpoint) -> Result<()> + 'static) -> Guard {
        HOOK.with(|slot| {
            assert!(slot.borrow().is_none());
            *slot.borrow_mut() = Some(Box::new(hook));
        });
        Guard
    }
    pub(super) fn reach(point: Checkpoint) -> Result<()> {
        HOOK.with(|slot| match slot.borrow_mut().as_mut() {
            Some(hook) => hook(point),
            None => Ok(()),
        })
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            HOOK.with(|slot| {
                slot.borrow_mut().take();
            });
        }
    }
}
