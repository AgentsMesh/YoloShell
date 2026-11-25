impl RemoteHost {
    pub async fn open_attachment(
        &self,
        terminal_id: Uuid,
        after_sequence: u64,
    ) -> Result<RemoteAttachment, RemoteError> {
        if !matches!(self.state, RemoteState::Ready(_)) {
            return Err(RemoteError::Disconnected);
        }
        let deadline = RemoteDeadline::operation();
        let request = RuntimeChannelRequest::attachment(self.runtime.clone());
        let opened = deadline.run(self.connection.open_runtime(&request)).await?;
        validate_channel(
            opened.identity,
            self.connection.info().connection_id,
            ChannelPurpose::TerminalAttachment,
        )?;
        let mut rpc = RemoteRpc::new(opened.stream);
        let initial: AttachResult = rpc
            .request_until(
                method::ATTACH_SESSION,
                &AttachRequest {
                    terminal_id,
                    after_seq: after_sequence,
                },
                deadline,
            )
            .await?;
        initial.validate_for_request(initial.runtime_epoch, after_sequence)?;
        Ok(RemoteAttachment {
            rpc,
            terminal_id,
            runtime_epoch: initial.runtime_epoch,
            initial,
        })
    }

    pub async fn run_command(
        &self,
        policy: &CommandPolicy,
        request: CommandRequest,
    ) -> Result<CommandOutput, RemoteError> {
        ensure_ready(&self.state)?;
        let command = policy.authorize(request)?;
        let outcome = self.connection.execute(&command).await?;
        validate_channel(
            outcome.identity,
            self.connection.info().connection_id,
            ChannelPurpose::Command,
        )?;
        outcome.value.validate(&command)?;
        Ok(outcome.value)
    }

    pub async fn read_file(
        &self,
        policy: &FileReadPolicy,
        request: FileReadRequest,
    ) -> Result<FileReadResult, RemoteError> {
        ensure_ready(&self.state)?;
        policy.preflight(&request)?;
        let canonical = self.connection.canonicalize(request.path()).await?;
        validate_channel(
            canonical.identity,
            self.connection.info().connection_id,
            ChannelPurpose::FileRead,
        )?;
        let authorized = policy.authorize(request, canonical.value)?;
        let outcome = self.connection.read_file(&authorized).await?;
        validate_channel(
            outcome.identity,
            self.connection.info().connection_id,
            ChannelPurpose::FileRead,
        )?;
        outcome.value.validate(&authorized)?;
        Ok(outcome.value)
    }
}

fn ensure_ready(state: &RemoteState) -> Result<(), RemoteError> {
    if matches!(state, RemoteState::Ready(_)) {
        Ok(())
    } else {
        Err(RemoteError::Disconnected)
    }
}
