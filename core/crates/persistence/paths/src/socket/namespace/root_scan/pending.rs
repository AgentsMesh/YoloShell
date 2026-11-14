use super::*;

impl RootScanner {
    pub(super) fn scan_pending(&mut self) -> Result<ScanChunk> {
        // Stage recovery can create .stage.lock. Serialize the entire chunk
        // against registered last-owner cleanup and directory replacement.
        let Some(mut lifecycle) =
            lifecycle_lock::SocketNamespaceLifecycleLock::try_acquire(&self.root)?
        else {
            return Ok(ScanChunk::Busy);
        };
        lifecycle.verify_root(&self.root)?;
        let pending = self.pending.as_ref().expect("pending stage scan exists");
        if self.root.revision()? != pending.revision {
            self.reset();
            return Ok(ScanChunk::Progress);
        }
        let pending = self.pending.as_mut().expect("pending stage scan exists");
        pending.authority.verify()?;
        let chunk = pending.scanner.scan_chunk()?;
        pending.authority.verify()?;
        lifecycle.verify_root(&self.root)?;
        if matches!(chunk, ScanChunk::PassComplete) {
            let removed = pending
                .authority
                .cleanup_empty(&self.root, &mut lifecycle)?;
            self.pending.take();
            if removed {
                self.reset();
            }
            return Ok(ScanChunk::Progress);
        }
        Ok(chunk)
    }
}
