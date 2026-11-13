use super::{file_root_ui_constants::*, management_tui_driver::*};

impl Driver {
    pub async fn file_roots_from_root(&mut self) {
        self.send(HOME).await;
        for _ in 0..ROOT_FILES_INDEX {
            self.send(DOWN).await;
        }
        self.send(ENTER).await;
        self.wait(|text| text.contains(ADD_FILE_ROOT)).await;
    }
    pub async fn edit_root_path(&mut self, path: &str) {
        self.file_roots_from_root().await;
        self.send(ENTER).await;
        self.wait(|text| text.contains(EDIT_FILE_ROOT)).await;
        self.send(ENTER).await;
        self.send(CLEAR_FIELD).await;
        self.field(path).await;
        self.send(ENTER).await;
        self.wait(|text| text.contains(DRAFT) && text.contains(ADD_FILE_ROOT))
            .await;
    }
    pub async fn reload_root_draft(&mut self, revision: u64) {
        self.send(HOME).await;
        for _ in 0..ROOT_RELOAD_INDEX {
            self.send(DOWN).await;
        }
        self.send(ENTER).await;
        self.wait(|text| text.contains(DISCARD)).await;
        self.send(TAB).await;
        self.send(ENTER).await;
        self.active(revision).await;
    }
}
