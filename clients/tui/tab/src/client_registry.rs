use crate::{TabDocument, constants::*, default_directory, store::is_document_decode_error};
use anyhow::{Result, ensure};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_paths::{
    directory::PrivateDirectory, directory_stream::DirectoryStream, namespace_lease::NamespaceLease,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientDirectory {
    pub id: Uuid,
    pub path: PathBuf,
}

/// Read-only snapshots. The control protocol must authenticate the owner before using a path.
pub fn client_directories(hub: HubId) -> Result<Vec<ClientDirectory>> {
    client_directories_in(&default_directory(hub)?, hub)
}

#[doc(hidden)]
pub fn client_directories_in(base: &Path, hub: HubId) -> Result<Vec<ClientDirectory>> {
    let base = yolo_paths::platform::platform_path(base);
    let parent = match PrivateDirectory::open(&base) {
        Ok(parent) => parent,
        Err(error) if io_kind(&error) == Some(ErrorKind::NotFound) => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut result = Vec::new();
    if bound_document(&parent, hub)? {
        result.push(ClientDirectory {
            id: hub.as_uuid(),
            path: base,
        });
    }
    result.extend(children(&parent, hub, false)?);
    parent.verify_path()?;
    Ok(result)
}

pub(crate) fn children(
    parent: &PrivateDirectory,
    hub: HubId,
    include_uninitialized: bool,
) -> Result<Vec<ClientDirectory>> {
    parent.verify_path()?;
    let path = parent.path().join(CLIENTS_DIRECTORY);
    let clients = match parent.open_descendant(&path) {
        Ok(clients) => clients,
        Err(error) if io_kind(&error) == Some(ErrorKind::NotFound) => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut stream = DirectoryStream::open(clients.descriptor())?;
    let mut result = Vec::new();
    let mut count = 0;
    while let Some(name) = stream.next_name()? {
        count += 1;
        ensure!(count <= MAX_DISCOVERY_ENTRIES, DISCOVERY_LIMIT);
        let Some(id) = name.to_str().and_then(client_id) else {
            continue;
        };
        if id == hub.as_uuid() {
            continue;
        }
        let path = clients.path().join(name);
        let child = match clients.open_descendant(&path) {
            Ok(child) => child,
            Err(error) if absent_or_indirect(&error) => continue,
            Err(error) => return Err(error),
        };
        child.verify_path()?;
        if bound_document(&child, hub)? || include_uninitialized {
            result.push(ClientDirectory { id, path });
        }
    }
    clients.verify_path()?;
    parent.verify_path()?;
    result.sort_by_key(|client| client.id);
    Ok(result)
}

fn bound_document(directory: &PrivateDirectory, hub: HubId) -> Result<bool> {
    let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory.try_clone()?)])?;
    let path = directory.path().join(TAB_FILE);
    if namespace.file_is_missing(&path)? {
        return Ok(false);
    }
    // Discovery validates current state without taking the writer lock or writing it.
    match namespace.read_json::<TabDocument>(&path) {
        Ok(document) => {
            document.validate(hub)?;
            Ok(true)
        }
        Err(error) if is_document_decode_error(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

fn client_id(name: &str) -> Option<Uuid> {
    let id = Uuid::parse_str(name).ok()?;
    (!id.is_nil() && id.to_string() == name).then_some(id)
}

pub(crate) fn io_kind(error: &anyhow::Error) -> Option<ErrorKind> {
    error.chain().find_map(|error| {
        error
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind)
    })
}

fn absent_or_indirect(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|error| error.downcast_ref::<std::io::Error>())
        .any(|error| {
            yolo_paths::directory::raw_errno_proves_path_identity_loss(error.raw_os_error())
        })
}
