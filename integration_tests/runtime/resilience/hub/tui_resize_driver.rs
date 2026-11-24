use super::Driver;

impl Driver {
    pub(crate) async fn resize(&mut self, cols: u16, rows: u16) {
        self.resize_pty(cols, rows).await;
        self.screen.screen_mut().set_size(rows, cols);
    }

    /// Resize the outer private PTY while keyboard bytes reach the real TUI.
    pub(crate) async fn resize_with_input(&mut self, cols: u16, rows: u16, bytes: &[u8]) {
        tokio::join!(self.resize_pty(cols, rows), self.send(bytes));
        self.screen.screen_mut().set_size(rows, cols);
    }

    async fn resize_pty(&self, cols: u16, rows: u16) {
        self.client
            .resize(
                self.target,
                self.grant.generation,
                &self.grant.writer_capability,
                cols,
                rows,
            )
            .await
            .unwrap();
    }
}
