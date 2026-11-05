use super::{memory_fs::MemoryFilesystem, server_command::contains};
use crate::remote::{
    constants::SSH_EXTENDED_DATA_STDERR, russh_backend::bundle_identity::UID_PROGRAM,
};
use russh::{ChannelId, server::Session};

pub(super) fn reply(
    filesystem: &MemoryFilesystem,
    command: &[u8],
    channel: ChannelId,
    session: &mut Session,
) -> Result<bool, russh::Error> {
    if !contains(command, UID_PROGRAM.as_bytes()) {
        return Ok(false);
    }
    let (stdout, stderr, status) = filesystem.uid_response();
    session.data(channel, stdout)?;
    if !stderr.is_empty() {
        session.extended_data(channel, SSH_EXTENDED_DATA_STDERR, stderr)?;
    }
    if let Some(status) = status {
        session.exit_status_request(channel, status)?;
    }
    session.eof(channel)?;
    session.close(channel)?;
    Ok(true)
}
