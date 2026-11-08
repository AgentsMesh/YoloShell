#[path = "sftp_server/prelude.rs"]
mod prelude;
use prelude::*;
#[path = "sftp_server/attributes.rs"]
mod attributes;
#[path = "sftp_server/directory.rs"]
mod directory;
#[path = "sftp_server/failures.rs"]
mod failures;
#[path = "sftp_server/handles.rs"]
mod handles;
#[path = "sftp_server/initialization.rs"]
mod initialization;
use directory::directory_handlers;

pub struct MemorySftp {
    filesystem: MemoryFilesystem,
    handles: HashMap<String, MemoryFile>,
    blocked_closes: HashSet<String>,
    reject_limits: bool,
    next_handle: u64,
}

impl russh_sftp::server::Handler for MemorySftp {
    type Error = StatusReply;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported.into()
    }

    async fn init(&mut self, _: u32, _: HashMap<String, String>) -> Result<Version, Self::Error> {
        let (version, reject_limits) = initialization::initialize(&self.filesystem).await?;
        self.reject_limits = reject_limits;
        Ok(version)
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        flags: OpenFlags,
        attrs: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        attributes::reject_unintended_mutations(&attrs)?;
        let handle = self.open_handle(&filename, flags, attrs)?;
        failures::stall_response(&self.filesystem, FailurePoint::SftpOpenResponsePending).await;
        Ok(Handle { id, handle })
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        self.close_handle(&handle).await?;
        Ok(ok(id))
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        length: u32,
    ) -> Result<Data, Self::Error> {
        Ok(Data {
            id,
            data: self.read_handle(&handle, offset, length).await?,
        })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        failures::reject_oversized_write(&self.filesystem)?;
        self.write_handle(&handle, offset, data)?;
        Ok(ok(id))
    }

    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        Ok(Attrs {
            id,
            attrs: self.filesystem.metadata(&path, true)?,
        })
    }

    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        if path == MEMORY_RUNTIME && self.filesystem.take_failure(FailurePoint::StableMetadata) {
            return Err(StatusCode::PermissionDenied.into());
        }
        Ok(Attrs {
            id,
            attrs: self.filesystem.metadata(&path, false)?,
        })
    }

    async fn fstat(&mut self, id: u32, handle: String) -> Result<Attrs, Self::Error> {
        failures::stall_fstat(&self.filesystem).await;
        Ok(Attrs {
            id,
            attrs: self.metadata_handle(&handle)?,
        })
    }

    async fn setstat(
        &mut self,
        id: u32,
        path: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        attributes::reject_unintended_mutations(&attrs)?;
        self.filesystem.chmod(&path, attrs.permissions)?;
        Ok(ok(id))
    }

    async fn fsetstat(
        &mut self,
        id: u32,
        handle: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        attributes::reject_unintended_mutations(&attrs)?;
        self.chmod_handle(&handle, attrs.permissions)?;
        Ok(ok(id))
    }

    async fn remove(&mut self, id: u32, filename: String) -> Result<Status, Self::Error> {
        self.filesystem.remove(&filename);
        Ok(ok(id))
    }

    directory_handlers!();

    async fn rename(
        &mut self,
        id: u32,
        oldpath: String,
        newpath: String,
    ) -> Result<Status, Self::Error> {
        self.filesystem.rename_no_replace(&oldpath, &newpath)?;
        Ok(ok(id))
    }

    async fn symlink(
        &mut self,
        id: u32,
        linkpath: String,
        targetpath: String,
    ) -> Result<Status, Self::Error> {
        let target = if self
            .filesystem
            .take_failure(FailurePoint::TemporaryLinkTarget)
        {
            MEMORY_OUTSIDE
        } else {
            &linkpath
        };
        self.filesystem.symlink(&targetpath, target);
        failures::stall_response(&self.filesystem, FailurePoint::SftpSymlinkResponsePending).await;
        Ok(ok(id))
    }

    async fn readlink(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        failures::stall_readlink(&self.filesystem).await;
        Ok(name(id, self.filesystem.readlink(&path)?))
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        if self.filesystem.take_canonicalize_failure() {
            std::future::pending::<()>().await;
        }
        Ok(name(id, self.filesystem.realpath(path)))
    }

    async fn extended(
        &mut self,
        id: u32,
        request: String,
        data: Vec<u8>,
    ) -> Result<Packet, Self::Error> {
        super::sftp_extensions::respond(
            &self.filesystem,
            &self.handles,
            self.reject_limits,
            id,
            &request,
            &data,
        )
        .await
        .map_err(Into::into)
    }
}
