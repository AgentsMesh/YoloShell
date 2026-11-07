use super::*;

impl RpcConnection {
    pub(super) async fn authenticate_local(
        &mut self,
        expected_home: &Path,
        socket: &Path,
        deadline: ClientDeadline,
    ) -> Result<(), ClientError> {
        let status: HostStatus = self
            .request_until(method::HOST_STATUS, &(), deadline)
            .await?;
        if status.protocol_version != crate::model::PROTOCOL_VERSION {
            return Err(ClientError::HostProtocolMismatch {
                expected: crate::model::PROTOCOL_VERSION,
                reported: status.protocol_version,
            });
        }
        let reported =
            canonical_home(&status.home).map_err(|message| ClientError::InvalidHostHome {
                home: status.home.clone(),
                message,
            })?;
        if reported != expected_home {
            return Err(ClientError::HostHomeMismatch {
                socket: socket.to_path_buf(),
                expected: expected_home.to_path_buf(),
                reported,
            });
        }
        self.authenticated_status = Some(status);
        Ok(())
    }
}
