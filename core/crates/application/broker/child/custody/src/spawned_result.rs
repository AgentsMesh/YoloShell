use super::failure::CombinedCleanupFailure;
use anyhow::Result;

pub(super) fn combine_cleanup_results(first: Result<()>, second: Result<()>) -> Result<()> {
    match (first, second) {
        (Ok(()), result) | (result, Ok(())) => result,
        (Err(primary), Err(cleanup)) => Err(combine_cleanup_failure(primary, Err(cleanup))),
    }
}

#[cfg(test)]
pub(super) fn combine_cleanup(primary: Result<()>, cleanup: Result<()>) -> Result<()> {
    combine_cleanup_results(primary, cleanup)
}

pub fn combine_cleanup_failure(primary: anyhow::Error, cleanup: Result<()>) -> anyhow::Error {
    match cleanup {
        Ok(()) => primary,
        Err(cleanup) => CombinedCleanupFailure::new(
            primary,
            cleanup,
            crate::constants::BROKER_SPAWN_COMPENSATION_FAILURE_MESSAGE,
        )
        .into(),
    }
}
