use super::Driver;
use yolo_tui_tab::TabDocument;

#[derive(Clone, Copy, Debug)]
pub enum DocumentPhase {
    TabChange,
    InitialUnboundTab,
    NewShellBinding,
    ExistingBindingAfterOutput,
    ReturnToOriginalTab,
    RestoredTab,
    TabEntryCreated,
}

const DOCUMENT_TIMEOUT: &str = "TUI tab document did not reach its expected state";

pub async fn fail(
    driver: &Driver,
    phase: DocumentPhase,
    last_document: Option<TabDocument>,
    last_read_error: Option<String>,
) -> ! {
    let observation = driver.diagnostic().await;
    panic!(
        "{DOCUMENT_TIMEOUT}: phase={phase:?} last_document={last_document:?} last_read_error={last_read_error:?} {observation}"
    );
}
