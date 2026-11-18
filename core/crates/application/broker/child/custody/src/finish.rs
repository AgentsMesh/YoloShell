use anyhow::Result;
use std::time::Instant;

pub fn finish_after_termination_delivery<T>(result: Result<T>, deadline: Instant) -> Result<T> {
    let generic = yolo_process::wait_for_detached_termination_delivery(deadline);
    let broker = crate::wait_for_termination_delivery(deadline);
    let cleanup = combine_cleanup(generic, broker);
    match result {
        Ok(value) => cleanup.map(|()| value),
        Err(primary) => Err(crate::combine_cleanup_failure(primary, cleanup)),
    }
}

fn combine_cleanup(first: Result<()>, second: Result<()>) -> Result<()> {
    match (first, second) {
        (Ok(()), result) | (result, Ok(())) => result,
        (Err(primary), Err(cleanup)) => Err(crate::combine_cleanup_failure(primary, Err(cleanup))),
    }
}
