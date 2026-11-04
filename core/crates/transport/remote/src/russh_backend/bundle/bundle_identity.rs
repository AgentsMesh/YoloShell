use super::RusshConnection;
use crate::remote::{
    ChannelPurpose, CommandPolicy, CommandRequest, MultiplexedSshConnection, RemoteError,
    transport::validate_channel,
};

pub(crate) const UID_PROGRAM: &str = "/usr/bin/id";
pub(crate) const UID_ARGUMENT: &str = "-u";
const MAX_UID_OUTPUT_BYTES: usize = 11;
const UID_LINE_END: &[u8] = b"\n";
const INVALID_UID: &str = "SSH user identity probe did not return one canonical numeric UID";
const SUCCESS_EXIT: u32 = 0;

impl RusshConnection {
    pub(super) async fn bundle_owner(&self) -> Result<u32, RemoteError> {
        let uid = self
            .authenticated_uid
            .get_or_try_init(|| async {
                let policy = CommandPolicy::new([UID_PROGRAM.into()])?;
                let command = policy.authorize(
                    CommandRequest::new(UID_PROGRAM, vec![UID_ARGUMENT.into()])
                        .with_limits(self.operation_timeout, MAX_UID_OUTPUT_BYTES),
                )?;
                let response = self.execute(&command).await?;
                validate_channel(
                    response.identity,
                    self.info.connection_id,
                    ChannelPurpose::Command,
                )?;
                response.value.validate(&command)?;
                if response.value.timed_out
                    || response.value.truncated
                    || response.value.exit_status != Some(SUCCESS_EXIT)
                    || !response.value.stderr.is_empty()
                {
                    return Err(invalid());
                }
                parse_uid(&response.value.stdout)
            })
            .await?;
        Ok(*uid)
    }
}

pub(super) fn parse_uid(bytes: &[u8]) -> Result<u32, RemoteError> {
    if bytes.len() > MAX_UID_OUTPUT_BYTES {
        return Err(invalid());
    }
    let digits = bytes.strip_suffix(UID_LINE_END).ok_or_else(invalid)?;
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return Err(invalid());
    }
    let text = std::str::from_utf8(digits).map_err(|_| invalid())?;
    let value: u32 = text.parse().map_err(|_| invalid())?;
    if value.to_string() != text {
        return Err(invalid());
    }
    Ok(value)
}

fn invalid() -> RemoteError {
    RemoteError::RuntimeIntegrity(INVALID_UID.into())
}
