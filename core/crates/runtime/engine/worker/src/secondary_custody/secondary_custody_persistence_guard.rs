use super::WorkerNamespace;
use crate::paths::Paths;
use anyhow::Result;

pub(crate) fn durable_guard<'a>(
    namespace: &'a WorkerNamespace,
    paths: &'a Paths,
    terminal_id: uuid::Uuid,
) -> Result<yolo_paths::durable_mutation::SessionDurableMutationGuard<'a>> {
    yolo_paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        paths,
        namespace.lease(),
        terminal_id,
    )
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../unit_tests/secondary_custody/secondary_custody_lifecycle_wait_tests.rs"]
mod tests;
