pub(crate) struct HostInstanceOwnership {
    publication: Option<SocketCleanup>,
    locks: Option<[InstanceLock; HOST_INSTANCE_LOCK_COUNT]>,
}

impl HostInstanceOwnership {
    pub(crate) fn new(locks: [InstanceLock; HOST_INSTANCE_LOCK_COUNT]) -> Self {
        Self {
            publication: None,
            locks: Some(locks),
        }
    }

    pub(crate) fn capture_publication(
        &mut self,
        publication: crate::paths::socket_activation::SocketPublication,
        socket_namespace: Arc<crate::paths::socket_namespace::SocketNamespace>,
    ) {
        assert!(
            self.publication.is_none(),
            "{SERVING_HOST_PUBLICATION_CAPTURED}"
        );
        self.publication = Some(SocketCleanup::capture(publication, socket_namespace));
    }

    pub(crate) fn configure_worker_command(&self, command: &mut std::process::Command) -> Result<()> {
        let locks = self.locks.as_ref().context(SERVING_HOST_RELEASED)?;
        let [home_lock, endpoint_lock] = locks;
        let fences = [
            home_lock.child_spawn_fence()?,
            endpoint_lock.child_spawn_fence()?,
        ];
        for fence in fences {
            fence.configure(command);
        }
        Ok(())
    }

    pub(crate) fn worker_lock_descriptors_for_spawn(
        &self,
    ) -> Result<[std::os::fd::RawFd; HOST_INSTANCE_LOCK_COUNT]> {
        let locks = self.locks.as_ref().context(SERVING_HOST_RELEASED)?;
        let [home_lock, endpoint_lock] = locks;
        let fences = [home_lock.child_spawn_fence()?, endpoint_lock.child_spawn_fence()?];
        for fence in fences {
            fence.rearm_close_on_exec()?;
        }
        Ok(fences.map(|fence| fence.descriptor()))
    }
}

impl Drop for HostInstanceOwnership {
    fn drop(&mut self) {
        // Identity inspection and name-based unlink must complete while both locks are held.
        drop(self.publication.take());
        drop(self.locks.take());
    }
}
