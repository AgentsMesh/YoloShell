use russh_sftp::protocol::{FileAttributes, StatusCode};

pub(super) fn reject_unintended_mutations(attributes: &FileAttributes) -> Result<(), StatusCode> {
    let mutates_unrequested_metadata = attributes.size.is_some()
        || attributes.uid.is_some()
        || attributes.gid.is_some()
        || attributes.atime.is_some()
        || attributes.mtime.is_some();
    if mutates_unrequested_metadata {
        Err(StatusCode::PermissionDenied)
    } else {
        Ok(())
    }
}
