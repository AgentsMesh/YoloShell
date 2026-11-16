use super::*;

const CREATE_SESSION_FAILURE_DETAIL: &str = "create runtime resilience test session";

impl Harness {
    pub(crate) async fn create_cat(&self, name: &str) -> yolo_runtime::TerminalView {
        let (view, creation_token, creation_capability) = self.create_cat_inner(name).await;
        self.finalize_creation_and_verify_replay(
            creation_token,
            view.manifest.terminal_id,
            creation_capability,
        )
        .await;
        view
    }

    pub(crate) async fn create_unfinalized_cat(&self, name: &str) -> yolo_runtime::TerminalView {
        let (view, creation_token, creation_capability) = self.create_cat_inner(name).await;
        self.creations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(
                view.manifest.terminal_id,
                (creation_token, creation_capability),
            );
        view
    }

    async fn create_cat_inner(&self, name: &str) -> (yolo_runtime::TerminalView, Uuid, Uuid) {
        let creation_token = Uuid::new_v4();
        let created = self
            .client()
            .create_session(&CreateTerminal {
                creation_token,
                name: Some(name.into()),
                cwd: self.home.path().to_path_buf(),
                cols: Some(90),
                rows: Some(25),
            })
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "{CREATE_SESSION_FAILURE_DETAIL}: {error}; host_stderr={}",
                    self.host_stderr_tail()
                )
            });
        let view = self.wait_for_created_view(created.terminal_id).await;
        self.track_view(&view);
        (view, creation_token, created.creation_capability)
    }

    pub(crate) async fn finalize_creation_and_verify_replay(
        &self,
        creation_token: Uuid,
        terminal_id: Uuid,
        creation_capability: Uuid,
    ) {
        let client = self.client();
        client
            .finalize_creation(creation_token, terminal_id, creation_capability)
            .await
            .unwrap();
        client
            .finalize_creation(creation_token, terminal_id, creation_capability)
            .await
            .unwrap();
    }

    pub(crate) fn creation_credentials(&self, terminal_id: Uuid) -> (Uuid, Uuid) {
        self.creations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())[&terminal_id]
    }
}
