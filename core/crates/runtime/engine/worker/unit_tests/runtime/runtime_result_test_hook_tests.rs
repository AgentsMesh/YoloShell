use super::*;

const IGNORED_FAILURE: &str = "ignored failure";
const WATCHED_FAILURE: &str = "watched failure";

#[test]
fn records_only_watched_runtime_failures() {
    let watched = Uuid::new_v4();
    let ignored = Uuid::new_v4();
    watch(watched);
    record(watched, &Err(anyhow::anyhow!(WATCHED_FAILURE)));
    record(ignored, &Err(anyhow::anyhow!(IGNORED_FAILURE)));
    assert_eq!(take(watched).as_deref(), Some(WATCHED_FAILURE));
    assert_eq!(take(ignored), None);
}
