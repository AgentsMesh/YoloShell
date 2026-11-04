impl Control {
    pub async fn resize(&mut self, size: TerminalSize) -> Result<(), RelayError> {
        let Some(authority) = &self.authority else {
            return Ok(());
        };
        if !authority.is_current(Instant::now()) {
            self.authority = None;
            return Ok(());
        }
        let grant = authority.grant;
        let deadline = request_deadline(authority);
        let mut client = self.take_call_client().await?;
        let result = client
            .call(
                self.origin(),
                Command::Resize {
                    request: ResizeRequest {
                        request_id: uuid::Uuid::new_v4(),
                        grant,
                        size,
                    },
                },
                deadline,
            )
            .await;
        self.return_call_client(client);
        self.accept(result)?;
        Ok(())
    }


    /// Sends the latest requested size under current control. Returns `None`
    /// when there is no current authority, matching the startup `resize` path.
    pub(crate) async fn spawn_resize(
        &mut self,
        size: TerminalSize,
    ) -> Result<Option<crate::lanes::ResizeTask>, RelayError> {
        let (grant, deadline) = match self.authority.as_ref() {
            Some(authority) if authority.is_current(Instant::now()) => {
                (authority.grant, request_deadline(authority))
            }
            _ => return Ok(None),
        };
        let mut client = self.take_call_client().await?;
        let origin = self.origin();
        Ok(Some(tokio::spawn(async move {
            let result = client
                .call(
                    origin,
                    Command::Resize {
                        request: ResizeRequest {
                            request_id: uuid::Uuid::new_v4(),
                            grant,
                            size,
                        },
                    },
                    deadline,
                )
                .await;
            (client, result, true, size)
        })))
    }
}
