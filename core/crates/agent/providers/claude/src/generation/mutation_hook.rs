use anyhow::Result;

#[cfg(test)]
type MutationFenceHook =
    Option<Box<dyn FnMut(yolo_paths::durable_mutation::DurableMutationFencePhase) -> Result<()>>>;

#[cfg(test)]
thread_local! {
    static MUTATION_FENCE_HOOK: std::cell::RefCell<MutationFenceHook> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(test)]
pub(crate) fn set_test_hook(
    hook: impl FnMut(yolo_paths::durable_mutation::DurableMutationFencePhase) -> Result<()> + 'static,
) {
    MUTATION_FENCE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
pub fn run(phase: yolo_paths::durable_mutation::DurableMutationFencePhase) -> Result<()> {
    MUTATION_FENCE_HOOK.with(|slot| {
        let mut hook = slot.borrow_mut().take();
        let result = hook.as_mut().map_or(Ok(()), |hook| hook(phase));
        if hook.is_some() {
            *slot.borrow_mut() = hook;
        }
        result
    })
}

#[cfg(not(test))]
pub fn run(_phase: yolo_paths::durable_mutation::DurableMutationFencePhase) -> Result<()> {
    Ok(())
}
