use super::*;

impl Paths {
    pub fn socket_dir(&self) -> Result<PathBuf> {
        Ok(self.socket_location()?.directory)
    }

    pub fn host_socket(&self) -> Result<PathBuf> {
        self.socket_location()?.host_socket()
    }

    pub fn hub_socket(&self) -> Result<PathBuf> {
        self.socket_location()?.hub_socket()
    }

    pub fn connector_socket(&self) -> Result<PathBuf> {
        self.socket_location()?.connector_socket()
    }

    pub fn worker_socket(&self, id: Uuid) -> Result<PathBuf> {
        self.socket_location()?.worker_socket(id)
    }

    pub fn custody_guardian_socket(&self, id: Uuid) -> Result<PathBuf> {
        self.socket_location()?.custody_guardian_socket(id)
    }

    pub fn retain_socket_namespace(&self) -> Result<SocketNamespace> {
        admission::retain(
            self.home(),
            SocketNamespaceLocation::locate(self.home(), self.socket_root_override())?,
        )
    }

    fn socket_location(&self) -> Result<SocketNamespaceLocation> {
        SocketNamespaceLocation::resolve(self.home(), self.socket_root_override())
    }
}
