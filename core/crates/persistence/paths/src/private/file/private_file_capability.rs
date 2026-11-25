impl OpenedPrivateFile {
    pub fn durable_version(
        &self,
        path: &Path,
        max_bytes: u64,
    ) -> Result<crate::paths::identity::DurableFileVersion> {
        let (version, live) = crate::paths::identity::DurableFileVersion::capture_file_bounded(
            &self.file, path, max_bytes,
        )?;
        anyhow::ensure!(
            live == self.identity,
            crate::ReadVersionChanged::state(path)
        );
        Ok(version)
    }

    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }

    pub fn identity(&self) -> FileIdentity {
        self.identity
    }

    pub fn into_parts(self) -> (File, FileIdentity) {
        (self.file, self.identity)
    }
}

impl TemporaryPrivateFile {
    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }

    pub fn identity(&self) -> FileIdentity {
        self.identity
    }
}
