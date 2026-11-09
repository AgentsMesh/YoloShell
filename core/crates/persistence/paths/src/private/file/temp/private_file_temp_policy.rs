#[derive(Clone, Copy)]
enum TargetLockPolicy<'a> {
    Acquire,
    HeldByCaller(&'a TargetLockAuthority<'a>),
    SerializedByCaller(&'a ParentLockAuthority<'a>),
}

impl PrivateFileEntry {
    fn temporary_with_policy<F, G, H>(
        &self,
        prefix: &str,
        suffix: &str,
        mut after_target_open: F,
        mut after_stage_open: G,
        mut before_truncate: H,
        target_lock: TargetLockPolicy<'_>,
    ) -> Result<TemporaryPrivateFile>
    where
        F: FnMut() -> Result<()>,
        G: FnMut() -> Result<()>,
        H: FnMut() -> Result<()>,
    {
        if let TargetLockPolicy::SerializedByCaller(authority) = target_lock {
            authority.require_entry(self)?;
        }
        for _ in 0..PRIVATE_FILE_TEMPORARY_ATTEMPTS {
            if let Some(temporary) = self.temporary_once_with_policy(
                prefix,
                suffix,
                &mut after_target_open,
                &mut after_stage_open,
                &mut before_truncate,
                target_lock,
            )? {
                return Ok(temporary);
            }
            super::private_file_temp_prepare::retry_temporary()?;
        }
        Err(std::io::Error::from(std::io::ErrorKind::WouldBlock))
            .context(PRIVATE_FILE_TEMPORARY_RETRY_ERROR)
    }

    fn temporary_once_with_policy<F, G, H>(
        &self,
        prefix: &str,
        suffix: &str,
        after_target_open: &mut F,
        after_stage_open: &mut G,
        before_truncate: &mut H,
        target_lock: TargetLockPolicy<'_>,
    ) -> Result<Option<TemporaryPrivateFile>>
    where
        F: FnMut() -> Result<()>,
        G: FnMut() -> Result<()>,
        H: FnMut() -> Result<()>,
    {
        let locked_previous = match self.open_optional_target(target_lock, after_target_open)? {
            OptionalTarget::Ready(previous) => previous,
            OptionalTarget::Retry => return Ok(None),
        };
        let (previous, previous_process_lock) = match locked_previous {
            Some((opened, process_lock)) => (Some(opened), process_lock),
            None => (None, None),
        };
        let previous_digest = previous
            .as_ref()
            .map(|opened| PrivateFileDigest::capture(&opened.file, opened.identity.length()))
            .transpose()?;
        if let Some(opened) = &previous {
            opened.identity.verify_file_bounded(
                &opened.file,
                opened.identity.length(),
                &self.path,
                u64::MAX,
            )?;
        }
        let name = active_name(&self.name, prefix, suffix)?;
        let slot = self.available_staging_slot(&name, prefix)?;
        let mut before_create = || Ok(());
        let Some((file, process_lock)) = self.prepare_active(
            &name,
            &slot,
            after_stage_open,
            before_truncate,
            &mut before_create,
        )?
        else {
            return Ok(None);
        };
        let identity = FileIdentity::inspect_file_bounded(&file, &self.path, u64::MAX)?;
        let digest = PrivateFileDigest::capture(&file, identity.length())?;
        #[cfg(test)]
        super::private_file_temp_hooks::run_after_stage_identity()?;
        if !self.active_identity_matches(&name, identity)? {
            return Ok(None);
        }
        Ok(Some(TemporaryPrivateFile {
            file,
            _process_lock: process_lock,
            identity,
            digest,
            previous_digest,
            name,
            _previous_process_lock: previous_process_lock,
            previous,
            slot,
        }))
    }

    fn open_optional_target<F>(
        &self,
        target_lock: TargetLockPolicy<'_>,
        after_open: &mut F,
    ) -> Result<OptionalTarget>
    where
        F: FnMut() -> Result<()>,
    {
        if let TargetLockPolicy::HeldByCaller(authority) = target_lock {
            #[cfg(test)]
            super::private_file_temp_hooks::run_before_target_open()?;
            let opened = authority.open_target(self, u64::MAX)?;
            after_open()?;
            authority.open_target(self, u64::MAX)?;
            return Ok(OptionalTarget::Ready(Some((opened, None))));
        }
        if entry_is_missing(&self.directory, &self.name)? {
            return Ok(OptionalTarget::Ready(None));
        }
        #[cfg(test)]
        super::private_file_temp_hooks::run_before_target_open()?;
        let opened = match self.open_stable(write_flags(), u64::MAX) {
            Ok(opened) => opened,
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => {
                return Ok(OptionalTarget::Retry);
            }
            Err(error) => return Err(error),
        };
        after_open()?;
        let process_lock = match target_lock {
            TargetLockPolicy::Acquire => {
                let Some(process_lock) =
                    super::private_file_temp_prepare::lock_or_retry(&opened.file)?
                else {
                    return Ok(OptionalTarget::Retry);
                };
                Some(process_lock)
            }
            TargetLockPolicy::SerializedByCaller(authority) => {
                authority.require_entry(self)?;
                None
            }
            TargetLockPolicy::HeldByCaller(_) => unreachable!(),
        };
        let current = self.inspect(u64::MAX)?;
        if current.is_some_and(|identity| identity.unchanged(opened.identity)) {
            Ok(OptionalTarget::Ready(Some((opened, process_lock))))
        } else {
            Ok(OptionalTarget::Retry)
        }
    }
}
