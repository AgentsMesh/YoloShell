use super::Driver;
use crate::hub::tui_queries::constants::*;

impl Driver {
    pub(super) async fn restore_bound_view(&self) {
        let saved: yolo_tui_tab::TabDocument = yolo_runtime::paths::read_json(
            &self
                .workspace_path()
                .join(yolo_tui_tab::TAB_FILE),
        )
        .unwrap();
        if saved
            .tabs
            .active()
            .view            .layout
            .panes()
            .iter()
            .any(|pane| pane.terminal.is_some())
        {
            self.send(ESCAPE).await;
        }
    }

    pub async fn open(&mut self, name: &str, access: &str) {
        if let Err(error) = self.try_open(name, access).await {
            panic!("{error}; {}", self.diagnostic().await);
        }
    }

    /// Preserve the normal input sequence and wait boundaries while allowing
    /// the caller to collect independent origin evidence after a failed wait.
    pub async fn try_open(&mut self, name: &str, access: &str) -> Result<(), String> {
        self.send(INVENTORY).await;
        self.output
            .wait(&mut self.screen, |text| text.contains(TERMINAL_PICKER_READY_SCREEN))
            .await?;
        self.send(SEARCH).await;
        self.output
            .wait(&mut self.screen, |text| text.contains(SEARCH_STATUS))
            .await?;
        let mut edit = vec![SEARCH_BACKSPACE; self.search_characters];
        edit.extend_from_slice(name.as_bytes());
        edit.extend_from_slice(SELECT_SEARCH_RESULT);
        self.send(&edit).await;
        self.search_characters = name.chars().count();
        let title = format!("{name}{access}");
        self.output
            .wait(&mut self.screen, |text| text.contains(&title))
            .await
    }

    pub async fn access(&mut self, name: &str, access: &str) {
        let title = format!("{name}{access}");
        self.wait(|text| text.contains(&title)).await;
    }
}
