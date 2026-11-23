struct DelayedReader {
    started: Option<mpsc::SyncSender<()>>,
    release: mpsc::Receiver<Vec<u8>>,
    exhausted: bool,
}

impl Read for DelayedReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.exhausted {
            return Ok(FIXTURE_EOF_BYTES);
        }
        if let Some(started) = self.started.take() {
            let _ = started.send(());
        }
        let data = self
            .release
            .recv()
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?;
        let length = data.len().min(buffer.len());
        buffer[..length].copy_from_slice(&data[..length]);
        self.exhausted = true;
        Ok(length)
    }
}
