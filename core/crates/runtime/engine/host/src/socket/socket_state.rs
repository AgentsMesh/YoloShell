#[cfg(unix)]
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SocketIdentity {
    device: u64,
    inode: u64,
}

pub(crate) enum SocketCleanup {
    Published {
        publication: crate::paths::socket_activation::SocketPublication,
        socket_namespace: Arc<crate::paths::socket_namespace::SocketNamespace>,
    },
    #[cfg(test)]
    Fixture {
        path: PathBuf,
        identity: SocketIdentity,
    },
}

pub(crate) struct ServingHost {
    maintenance: Option<crate::maintenance::MaintenanceWorker>,
    host: Option<Arc<Host>>,
}

impl ServingHost {
    pub(crate) fn capture(
        mut host: Host,
        publication: crate::paths::socket_activation::SocketPublication,
    ) -> Self {
        let socket_namespace = Arc::clone(&host.socket_namespace);
        host.instance_ownership
            .capture_publication(publication, socket_namespace);
        let host = Arc::new(host);
        crate::host::agent_watch::spawn(&host);
        Self {
            maintenance: None,
            host: Some(host),
        }
    }

    pub(crate) fn host(&self) -> &Arc<Host> {
        self.host.as_ref().expect(SERVING_HOST_RELEASED)
    }

    pub(crate) fn start_maintenance(&mut self) -> Result<()> {
        let socket_scanner = self.host().socket_namespace.scanner()?;
        let plan = crate::maintenance::maintenance_plan_with_socket_scanner(socket_scanner)?;
        self.maintenance = Some(crate::maintenance::MaintenanceWorker::start(plan));
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn start_maintenance_plan(&mut self, plan: crate::maintenance::MaintenancePlan) {
        self.maintenance = Some(crate::maintenance::MaintenanceWorker::start(plan));
    }

    pub(crate) fn take_maintenance(&mut self) -> Option<crate::maintenance::MaintenanceWorker> {
        self.maintenance.take()
    }

    pub(crate) fn release(&mut self) {
        drop(self.maintenance.take());
        drop(self.host.take());
    }
}

impl Drop for ServingHost {
    fn drop(&mut self) {
        self.release();
    }
}

impl SocketCleanup {
    pub(crate) fn capture(
        publication: crate::paths::socket_activation::SocketPublication,
        socket_namespace: Arc<crate::paths::socket_namespace::SocketNamespace>,
    ) -> Self {
        Self::Published {
            publication,
            socket_namespace,
        }
    }

    #[cfg(test)]
    pub(crate) fn new(path: &Path) -> Result<Self> {
        bound_socket_identity(path)
            .map(|identity| Self::Fixture {
                path: path.to_path_buf(),
                identity,
            })
            .map_err(Into::into)
    }
}

impl Drop for SocketCleanup {
    fn drop(&mut self) {
        match self {
            Self::Published {
                publication,
                socket_namespace,
            } => {
                let _ = (publication, socket_namespace);
            }
            #[cfg(test)]
            Self::Fixture { path, identity } => {
                if bound_socket_identity(path).ok() == Some(*identity) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }
}
