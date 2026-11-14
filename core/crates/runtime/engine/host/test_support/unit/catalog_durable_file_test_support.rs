use std::cell::RefCell;

const DURABLE_READ_HOOK_ALREADY_INSTALLED: &str = "durable read test hook already installed";
const DURABLE_READ_HOOK_MISSING: &str = "selected durable read test hook exists";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DurableReadStage {
    AfterDescriptorVerification,
    AfterPublishedVerification,
    BeforeFinalRead,
}

type DurableReadHook = Box<dyn FnOnce(&Path) -> Result<()>>;

thread_local! {
    static DURABLE_READ_HOOK: RefCell<Option<(DurableReadStage, DurableReadHook)>> =
        RefCell::new(None);
}

pub(crate) struct DurableReadHookGuard;

pub(crate) fn install_durable_read_hook(
    stage: DurableReadStage,
    hook: impl FnOnce(&Path) -> Result<()> + 'static,
) -> DurableReadHookGuard {
    DURABLE_READ_HOOK.with(|slot| {
        let previous = slot.replace(Some((stage, Box::new(hook))));
        assert!(previous.is_none(), "{DURABLE_READ_HOOK_ALREADY_INSTALLED}");
    });
    DurableReadHookGuard
}

pub(crate) fn run_durable_read_hook(stage: DurableReadStage, path: &Path) -> Result<()> {
    let hook = DURABLE_READ_HOOK.with(|slot| {
        let selected = slot
            .borrow()
            .as_ref()
            .is_some_and(|(candidate, _)| *candidate == stage);
        selected.then(|| slot.borrow_mut().take().expect(DURABLE_READ_HOOK_MISSING).1)
    });
    hook.map_or(Ok(()), |hook| hook(path))
}

impl Drop for DurableReadHookGuard {
    fn drop(&mut self) {
        DURABLE_READ_HOOK.with(|slot| {
            slot.borrow_mut().take();
        });
    }
}
