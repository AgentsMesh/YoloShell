use crate::constants::*;
use anyhow::{Context, Result};
use tokio::{
    net::UnixListener,
    time::{Instant, timeout_at},
};
use uuid::Uuid;
use yolo_paths::{
    Paths,
    coordination::CoordinationLock,
    identity::FileIdentity,
    namespace_lease::NamespaceLease,
    private_file::PrivateFileEntry,
    socket_activation::{self, SocketPublication},
    socket_namespace::SocketNamespace,
};
use yolo_tui_tab::TabStore;
use yolo_tui_client_control_protocol::ControlDescriptor;

pub(crate) struct Publication {
    descriptor: PrivateFileEntry,
    identity: Option<FileIdentity>,
    socket: Option<SocketPublication>,
    _namespace: SocketNamespace,
    _lock: CoordinationLock,
}

impl Publication {
    pub async fn bind(
        store: &TabStore,
        deadline: Instant,
    ) -> Result<(Self, UnixListener, Uuid)> {
        let directory = store.directory()?;
        let lock = CoordinationLock::try_acquire(
            &directory.path().join(CONTROL_OWNER_LOCK),
            CONTROL_DESCRIPTOR_MAX_BYTES,
        )?
        .context(CONTROL_BUSY)?;
        let namespace = Paths::new(directory.path()).retain_socket_namespace()?;
        let endpoint = Uuid::new_v4();
        let token = Uuid::new_v4();
        let path = directory.path().join(CONTROL_DESCRIPTOR_NAME);
        let descriptor = PrivateFileEntry::from_directory(directory.try_clone()?, &path)?;
        let expected = descriptor.inspect(CONTROL_DESCRIPTOR_MAX_BYTES)?;
        if expected.is_some() {
            let previous = crate::endpoint::DescriptorRead::open(directory.path())?;
            descriptor.verify_expected(Some(previous.identity), CONTROL_DESCRIPTOR_MAX_BYTES)?;
        }
        let lease = NamespaceLease::from_directories(vec![(CONTROL_NAMESPACE, directory)])?;
        let (listener, socket) = timeout_at(
            deadline,
            socket_activation::publish(
                namespace.directory()?,
                &namespace.worker_socket(endpoint)?,
                CONTROL_REQUEST_TIMEOUT,
                CONTROL_BUSY,
            ),
        )
        .await
        .context(CONTROL_DEADLINE)??;
        let mut publication = Self {
            descriptor,
            identity: None,
            socket: Some(socket),
            _namespace: namespace,
            _lock: lock,
        };
        let identity = lease.write_json_expected_with_commit_observer(
            &path,
            expected,
            &ControlDescriptor::new(endpoint, token),
            || store.directory().map(drop),
            |identity| publication.identity = Some(identity),
        )?;
        // The commit callback runs at rename with the pre-rename identity.
        // Successful publication returns the final identity including its updated ctime.
        publication.identity = Some(identity);
        Ok((publication, listener, token))
    }

    pub fn withdraw(&mut self) -> Result<()> {
        let result = self.withdraw_owned();
        if result.is_err() {
            self._namespace.preserve_on_release();
        }
        result
    }

    fn withdraw_owned(&mut self) -> Result<()> {
        if let Some(identity) = self.identity {
            self.descriptor
                .remove_if_exact(identity, CONTROL_DESCRIPTOR_MAX_BYTES)?;
            self.identity = None;
        }
        if let Some(socket) = self.socket.take() {
            socket.withdraw()?;
        }
        Ok(())
    }
}

impl Drop for Publication {
    fn drop(&mut self) {
        let _ = self.withdraw();
    }
}
