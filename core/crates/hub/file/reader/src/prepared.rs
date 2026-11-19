use crate::{anchor::Anchor, constants::*, descriptor::io_error, root::ReadRoot, stamp::FileStamp};
use std::ffi::CString;
use std::fs::File;
use std::os::unix::fs::FileExt;
use yolo_crypto::Sha256Digest;
use yolo_hub_file_protocol::{
    FILE_PROTOCOL_SCHEMA_VERSION, ReadFileRequest, ReadFileResult, ReadFileResultFields,
};
use yolo_hub_terminal_protocol::PublicError;

pub struct PreparedRead {
    parent: Anchor,
    protected: Vec<Anchor>,
    name: CString,
    file: File,
    stamp: FileStamp,
    request: ReadFileRequest,
    version: Sha256Digest,
}

impl PreparedRead {
    pub(crate) fn open(root: &ReadRoot, request: &ReadFileRequest) -> Result<Self, PublicError> {
        let mut components = request
            .fields()
            .relative_path
            .as_str()
            .split(PATH_SEPARATOR)
            .collect::<Vec<_>>();
        let name = CString::new(components.pop().ok_or(PublicError::InvalidMetadata)?)
            .map_err(|_| PublicError::InvalidMetadata)?;
        let mut parent = root.anchor.clone_anchor()?;
        for component in components {
            parent.push(component)?;
        }
        let file = parent.open(&name, FILE_FLAGS)?;
        let stamp = FileStamp::capture(&file)?;
        let fields = request.fields();
        if fields.offset > stamp.length {
            return Err(PublicError::InvalidRequest);
        }
        let version = Sha256Digest::capture(&(
            FILE_VERSION_DOMAIN,
            fields.target,
            &fields.relative_path,
            &stamp,
        ))
        .map_err(|_| PublicError::Internal)?;
        if fields
            .expected_version
            .is_some_and(|expected| expected != version)
        {
            return Err(PublicError::Conflict);
        }
        let protected = root
            .protected
            .iter()
            .map(Anchor::clone_anchor)
            .collect::<Result<_, _>>()?;
        let read = Self {
            parent,
            protected,
            name,
            file,
            stamp,
            request: request.clone(),
            version,
        };
        read.verify()?;
        Ok(read)
    }

    pub fn read(self) -> Result<ReadFileResult, PublicError> {
        self.verify()?;
        let fields = self.request.fields();
        let length = (self.stamp.length - fields.offset).min(u64::from(fields.length));
        let mut data = vec![0; length as usize];
        self.file
            .read_exact_at(&mut data, fields.offset)
            .map_err(io_error)?;
        self.verify()?;
        let result = ReadFileResult::new(ReadFileResultFields {
            schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
            target: fields.target,
            relative_path: fields.relative_path.clone(),
            offset: fields.offset,
            version: self.version,
            total_length: self.stamp.length,
            eof: fields.offset + length == self.stamp.length,
            data,
        })?;
        result.validate_for(&self.request)?;
        Ok(result)
    }

    fn verify(&self) -> Result<(), PublicError> {
        self.parent.verify()?;
        for protected in &self.protected {
            protected.verify()?;
            if self.parent.overlaps(protected) {
                return Err(PublicError::Unauthorized);
            }
        }
        let named = self
            .parent
            .open(&self.name, FILE_FLAGS)
            .map_err(|_| PublicError::Conflict)?;
        if FileStamp::capture(&self.file)? != self.stamp
            || FileStamp::capture(&named)? != self.stamp
        {
            return Err(PublicError::Conflict);
        }
        Ok(())
    }
}
