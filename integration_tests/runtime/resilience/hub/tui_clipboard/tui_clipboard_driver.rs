use super::Driver;

impl Driver {
    /// An independent read-only tap preserves the driver's screen observation.
    pub async fn clipboard_output(&self) -> yolo_client::Attachment {
        self.client
            .attach(
                self.target.terminal_id,
                yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE,
            )
            .await
            .unwrap()
    }
}
