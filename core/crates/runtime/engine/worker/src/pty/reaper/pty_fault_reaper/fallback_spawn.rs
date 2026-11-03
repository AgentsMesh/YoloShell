use super::{
    FALLBACK_OWNER_SPAWN_PANIC_DETAIL, FALLBACK_OWNER_THREAD_NAME, FallbackTask, TaskSlot,
};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

pub(crate) struct OwnerSpawnFailure {
    pub(crate) task: FallbackTask,
    pub(crate) error: io::Error,
}

pub(crate) fn spawn_owner(task: FallbackTask) -> std::result::Result<(), OwnerSpawnFailure> {
    spawn_owner_with(task, |slot| {
        std::thread::Builder::new()
            .name(FALLBACK_OWNER_THREAD_NAME.to_owned())
            .spawn(move || run_slot(slot))
            .map(drop)
    })
}

pub(crate) fn spawn_owner_with<Spawn>(
    task: FallbackTask,
    spawn: Spawn,
) -> std::result::Result<(), OwnerSpawnFailure>
where
    Spawn: FnOnce(TaskSlot) -> io::Result<()>,
{
    let slot = Arc::new(Mutex::new(Some(task)));
    match catch_unwind(AssertUnwindSafe(|| spawn(Arc::clone(&slot)))) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => recover_spawn(&slot, error),
        Err(payload) => {
            let detail = crate::worker::supervision::panic_detail(payload);
            recover_spawn(
                &slot,
                io::Error::other(format!("{FALLBACK_OWNER_SPAWN_PANIC_DETAIL}: {detail}")),
            )
        }
    }
}

pub(crate) fn recover_spawn(
    slot: &TaskSlot,
    error: io::Error,
) -> std::result::Result<(), OwnerSpawnFailure> {
    match take_slot(slot) {
        Some(task) => Err(OwnerSpawnFailure { task, error }),
        None => Ok(()),
    }
}

pub(crate) fn run_slot(slot: TaskSlot) {
    if let Some(task) = take_slot(&slot) {
        task();
    }
}

pub(crate) fn take_slot(slot: &TaskSlot) -> Option<FallbackTask> {
    slot.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
}
