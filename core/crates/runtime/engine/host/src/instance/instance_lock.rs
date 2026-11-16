pub(crate) struct InstanceLock {
    lock: CoordinationLock,
}

#[derive(Debug)]
pub struct HostInstanceBusy {
    pub(crate) resource: String,
    pub(crate) path: PathBuf,
}

impl std::fmt::Display for HostInstanceBusy {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            output,
            "another Yolo host already owns the {} guarded by {}",
            self.resource,
            self.path.display()
        )
    }
}

impl std::error::Error for HostInstanceBusy {}

impl InstanceLock {
    pub(crate) fn child_spawn_fence(
        &self,
    ) -> Result<crate::coordination::CoordinationChildSpawnFence> {
        self.lock.child_spawn_fence()
    }

    pub(crate) fn acquire(path: &Path, resource: &str) -> Result<Self> {
        Self::acquire_inner(path, resource, || Ok(()))
    }

    #[cfg(test)]
    pub(crate) fn acquire_with_hook<F>(path: &Path, resource: &str, after_lock: F) -> Result<Self>
    where
        F: FnOnce() -> Result<()>,
    {
        Self::acquire_inner(path, resource, after_lock)
    }

    pub(crate) fn acquire_inner<F>(path: &Path, resource: &str, after_lock: F) -> Result<Self>
    where
        F: FnOnce() -> Result<()>,
    {
        let acquired = {
            #[cfg(test)]
            {
                crate::paths::test_support::acquire_coordination_with_hook(
                    path,
                    HOST_INSTANCE_LOCK_MAX_BYTES,
                    after_lock,
                )
            }
            #[cfg(not(test))]
            {
                let _ = after_lock;
                CoordinationLock::try_acquire(path, HOST_INSTANCE_LOCK_MAX_BYTES)
            }
        }?;
        let Some(mut lock) = acquired else {
            return Err(HostInstanceBusy {
                resource: resource.to_owned(),
                path: path.to_path_buf(),
            }
            .into());
        };
        let record = format!("{HOST_INSTANCE_LOCK_PID_PREFIX}{}\n", std::process::id());
        lock.replace_contents(record.as_bytes())
            .with_context(|| format!("publish host instance lock {}", path.display()))?;
        Ok(Self { lock })
    }
}
