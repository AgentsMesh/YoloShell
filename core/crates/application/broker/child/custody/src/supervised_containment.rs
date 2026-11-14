use anyhow::Result;
use yolo_broker_supervision_state::SupervisionPaths;
use yolo_broker_supervision_state::SupervisionRecord;

pub(crate) fn contain(paths: &SupervisionPaths, record: &SupervisionRecord) -> Result<()> {
    paths.contain_broker(record)
}

pub(super) fn combine(primary: Result<()>, cleanup: Result<()>) -> Result<()> {
    match (primary, cleanup) {
        (Ok(()), result) | (result, Ok(())) => result,
        (Err(primary), Err(cleanup)) => Err(crate::pending::combine_cleanup_failure(
            primary,
            Err(cleanup),
        )),
    }
}
