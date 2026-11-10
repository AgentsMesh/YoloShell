use super::*;

impl ProcessLockTurn {
    pub(crate) fn queued_waiters_for_test(key: ObjectIdentity) -> usize {
        current_registry()
            .turns
            .get(&key)
            .map_or(usize::default(), |state| state.waiters.len())
    }
}
