use super::{Driver, constants::*, observe};
use crate::Harness;
use yolo_runtime::model::{TerminalRuntimeStatus, TerminalView};

pub(super) async fn completed_terminal_moves_to_history(
    driver: &mut Driver,
    host: &Harness,
    original: &TerminalView,
) {
    driver.send(TERMINAL_PICKER_KEYS).await;
    driver.wait(active_is_empty).await;
    let ended = observe::single_shell(host).await;
    assert_eq!(ended.manifest.terminal_id, original.manifest.terminal_id);
    assert!(!ended.manifest.archived);
    let runtime = ended.runtime.as_ref().unwrap();
    assert_eq!(runtime.status, TerminalRuntimeStatus::Exited);
    assert_eq!(
        runtime.runtime_epoch,
        original.runtime.as_ref().unwrap().runtime_epoch
    );
    assert!(runtime.exit.is_some());

    driver.send(HISTORY_KEY).await;
    let title = runtime
        .presentation
        .title
        .as_deref()
        .filter(|title| !title.is_empty())
        .unwrap_or(&ended.manifest.name);
    let row = format!("{title}{ENDED_LABEL}");
    driver
        .wait(|text| {
            text.contains(SINGLE_TERMINAL_HEADER)
                && text.contains(CURRENT_CATALOGUE)
                && text.contains(HISTORY_SCOPE)
                && text.contains(&row)
        })
        .await;
    driver.send(HISTORY_KEY).await;
    driver.wait(active_is_empty).await;
    let retained = observe::single_shell(host).await;
    assert_same_completion(&retained, &ended);
}

pub(super) fn assert_same_completion(retained: &TerminalView, ended: &TerminalView) {
    assert_eq!(retained.manifest.terminal_id, ended.manifest.terminal_id);
    assert_eq!(retained.manifest.name, ended.manifest.name);
    assert_eq!(retained.manifest.archived, ended.manifest.archived);
    let (retained, ended) = (
        retained.runtime.as_ref().unwrap(),
        ended.runtime.as_ref().unwrap(),
    );
    assert_eq!(retained.runtime_epoch, ended.runtime_epoch);
    assert_eq!(retained.status, TerminalRuntimeStatus::Exited);
    assert_eq!(retained.exit, ended.exit);
}

fn active_is_empty(text: &str) -> bool {
    text.contains(EMPTY_TERMINAL_HEADER)
        && text.contains(CURRENT_CATALOGUE)
        && text.contains(ACTIVE_SCOPE)
        && text.contains(EMPTY_TERMINAL_LIST)
}
