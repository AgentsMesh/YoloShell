use super::*;

impl MemoryFilesystem {
    pub fn open(
        &self,
        path: &str,
        flags: OpenFlags,
        attrs: FileAttributes,
    ) -> Result<MemoryFile, StatusCode> {
        let mut state = self.0.lock().unwrap();
        if flags.contains(OpenFlags::EXCLUDE) && entry_exists(&state, path) {
            return Err(StatusCode::Failure);
        }
        if flags.contains(OpenFlags::CREATE) && !state.files.contains_key(path) {
            state.files.insert(
                path.into(),
                MemoryFile::new(&[], attrs.permissions.unwrap_or(RUNTIME_UPLOAD_MODE)),
            );
        }
        let resolved = state
            .links
            .get(path)
            .cloned()
            .unwrap_or_else(|| path.into());
        let file = state
            .files
            .get(&resolved)
            .cloned()
            .ok_or(StatusCode::NoSuchFile)?;
        if flags.contains(OpenFlags::TRUNCATE) {
            file.0.lock().unwrap().data.clear();
        }
        Ok(file)
    }

    pub fn metadata(&self, path: &str, follow: bool) -> Result<FileAttributes, StatusCode> {
        if self.take_failure(FailurePoint::Metadata) {
            return Err(StatusCode::PermissionDenied);
        }
        let state = self.0.lock().unwrap();
        let resolved = if follow {
            state.links.get(path).map(String::as_str).unwrap_or(path)
        } else {
            path
        };
        if !follow && state.links.contains_key(path) {
            let mut attributes = objects::attributes(EMPTY_ENTRY_SIZE, RUNTIME_EXECUTABLE_MODE);
            attributes.set_symlink(true);
            return Ok(attributes);
        }
        if let Some(permissions) = state.directories.get(resolved) {
            let mut attributes = objects::attributes(EMPTY_ENTRY_SIZE, *permissions);
            attributes.set_dir(true);
            return Ok(attributes);
        }
        let file = state.files.get(resolved).ok_or(StatusCode::NoSuchFile)?;
        Ok(objects::file_attributes(file))
    }

    pub fn rename_no_replace(&self, old: &str, new: &str) -> Result<(), StatusCode> {
        if self.take_failure(FailurePoint::ReleaseRename) {
            return Err(StatusCode::PermissionDenied);
        }
        let mut state = self.0.lock().unwrap();
        if entry_exists(&state, new) {
            return Err(StatusCode::Failure);
        }
        if let Some(entry) = state.files.remove(old) {
            state.files.insert(new.into(), entry);
            return Ok(());
        }
        if let Some(target) = state.links.remove(old) {
            state.links.insert(new.into(), target);
            state.activations += COUNTER_INCREMENT;
            return Ok(());
        }
        Err(StatusCode::NoSuchFile)
    }

    pub fn posix_rename(&self, old: &str, new: &str) -> Result<(), StatusCode> {
        if self.take_failure(FailurePoint::ActivationRename) {
            return Err(StatusCode::PermissionDenied);
        }
        let mut state = self.0.lock().unwrap();
        let target = state.links.remove(old).ok_or(StatusCode::NoSuchFile)?;
        state.files.remove(new);
        state.directories.remove(new);
        state.links.insert(new.into(), target);
        state.activations += COUNTER_INCREMENT;
        Ok(())
    }

    pub fn symlink(&self, link: &str, target: &str) {
        self.0
            .lock()
            .unwrap()
            .links
            .insert(link.into(), target.into());
    }

    pub fn readlink(&self, path: &str) -> Result<String, StatusCode> {
        self.0
            .lock()
            .unwrap()
            .links
            .get(path)
            .cloned()
            .ok_or(StatusCode::NoSuchFile)
    }

    pub fn remove(&self, path: &str) {
        let mut state = self.0.lock().unwrap();
        state.files.remove(path);
        state.links.remove(path);
    }

    pub fn realpath(&self, path: String) -> String {
        self.0
            .lock()
            .unwrap()
            .links
            .get(&path)
            .cloned()
            .unwrap_or(path)
    }
}

fn entry_exists(state: &State, path: &str) -> bool {
    state.files.contains_key(path)
        || state.directories.contains_key(path)
        || state.links.contains_key(path)
}
