use super::{Driver, RunningHub, constants::*, deadline};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::LeaseObservation;
use yolo_tui_tab::{TAB_FILE, TabDocument};
#[path = "../tui_driver/tui_document_diagnostics.rs"]
mod diagnostics;
pub use diagnostics::DocumentPhase;

pub async fn document(
    driver: &Driver,
    hub: &RunningHub,
    matches: impl Fn(&TabDocument) -> bool,
) -> TabDocument {
    document_at(driver, hub, DocumentPhase::TabChange, matches).await
}

pub async fn document_at(
    driver: &Driver,
    hub: &RunningHub,
    phase: DocumentPhase,
    matches: impl Fn(&TabDocument) -> bool,
) -> TabDocument {
    let mut last_document = None;
    let mut last_read_error = None;
    let result = tokio::time::timeout_at(deadline(), async {
        loop {
            match yolo_runtime::paths::read_json::<TabDocument>(
                &driver.workspace_path().join(TAB_FILE),
            ) {
                Ok(saved) => {
                    saved.validate(hub.hub).unwrap();
                    last_read_error = None;
                    if matches(&saved) {
                        return saved;
                    }
                    // Retain the typed document; format only on failure.
                    last_document = Some(saved);
                }
                Err(error) => last_read_error = Some(error.to_string()),
            }
            tokio::time::sleep(yolo_terminal_query_fixture::constants::POLL).await;
        }
    })
    .await;
    match result {
        Ok(saved) => saved,
        Err(_) => diagnostics::fail(driver, phase, last_document, last_read_error).await,
    }
}

pub async fn lease(hub: &RunningHub, target: RuntimeTarget) -> Option<LeaseObservation> {
    let Reply::LeaseObservation { lease } = hub
        .client()
        .await
        .call(hub.hub, Command::ObserveLease { target }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    lease
}

pub async fn no_writer(hub: &RunningHub, target: RuntimeTarget) {
    tokio::time::timeout_at(deadline(), async {
        while lease(hub, target).await.is_some() {
            tokio::time::sleep(yolo_terminal_query_fixture::constants::POLL).await;
        }
    })
    .await
    .unwrap();
}

pub async fn click(driver: &mut Driver, label: &str) -> (u16, u16) {
    driver.wait(|text| text.contains(label)).await;
    let screen = driver.screen();
    let (rows, columns) = screen.size();
    let width = label.chars().count() as u16;
    let (x, y) = (INVENTORY_TOP_ROW..rows)
        .flat_map(|y| (0..columns.min(INVENTORY_COLUMNS)).map(move |x| (x, y)))
        .find(|(x, y)| {
            *x + width <= INVENTORY_COLUMNS
                && label.chars().enumerate().all(|(index, c)| {
                    screen
                        .cell(*y, *x + index as u16)
                        .is_some_and(|cell| cell.contents() == c.to_string())
                })
        })
        .unwrap();
    mouse(driver, MOUSE_LEFT, x, y, MOUSE_DOWN).await;
    mouse(driver, MOUSE_LEFT, x, y, MOUSE_UP).await;
    (x, y)
}

pub async fn mouse(driver: &Driver, button: u16, x: u16, y: u16, suffix: char) {
    driver
        .send(
            format!(
                "{MOUSE_PREFIX}{button};{};{}{suffix}",
                x + MOUSE_COORDINATE_BASE,
                y + MOUSE_COORDINATE_BASE
            )
            .as_bytes(),
        )
        .await;
}

pub async fn runtime_unchanged(
    host: &crate::Harness,
    target: RuntimeTarget,
    before: &yolo_runtime::TerminalView,
) {
    let after = host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let (left, right) = (
        before.runtime.as_ref().unwrap(),
        after.runtime.as_ref().unwrap(),
    );
    assert_eq!(left.runtime_epoch, right.runtime_epoch);
    assert_eq!(left.worker_id, right.worker_id);
    assert_eq!(
        (left.worker_pid, left.worker_start_time),
        (right.worker_pid, right.worker_start_time)
    );
    assert_eq!(
        (left.child_pid, left.child_start_time),
        (right.child_pid, right.child_start_time)
    );
    assert!(!after.manifest.archived);
    assert_eq!(right.status, yolo_runtime::model::TerminalRuntimeStatus::Running);
}

pub fn assert_retained_layouts(
    before: &yolo_tui_tab::TabCatalog,
    after: &yolo_tui_tab::TabCatalog,
) {
    assert_eq!(before.active().id, after.active().id);
    assert_eq!(before.entries().len(), after.entries().len());
    for (before, after) in before.entries().iter().zip(after.entries()) {
        assert_eq!(before.id, after.id);
        assert_eq!(before.name, after.name);
        assert_eq!(before.view.layout, after.view.layout);
    }
}
