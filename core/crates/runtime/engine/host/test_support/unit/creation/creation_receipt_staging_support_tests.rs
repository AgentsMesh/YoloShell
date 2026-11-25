use anyhow::Result;
use std::cell::RefCell;
use std::path::Path;

const ALREADY_INSTALLED_DETAIL: &str = "receipt staging test hook is already installed";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Stage {
    AfterReadOpen,
    BeforeCreate,
    BeforeSlotMove,
}

type Hook = Box<dyn FnMut(Stage, &Path) -> Result<()>>;

thread_local! {
    static TEST_HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) };
}

pub(crate) struct Guard(std::marker::PhantomData<std::rc::Rc<()>>);

pub(crate) fn install(hook: impl FnMut(Stage, &Path) -> Result<()> + 'static) -> Guard {
    TEST_HOOK.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "{ALREADY_INSTALLED_DETAIL}");
    });
    Guard(std::marker::PhantomData)
}

pub(crate) fn run(stage: Stage, path: &Path) -> Result<()> {
    TEST_HOOK.with(|slot| match slot.borrow_mut().as_mut() {
        Some(hook) => hook(stage, path),
        None => Ok(()),
    })
}

impl Drop for Guard {
    fn drop(&mut self) {
        TEST_HOOK.with(|slot| *slot.borrow_mut() = None);
    }
}
