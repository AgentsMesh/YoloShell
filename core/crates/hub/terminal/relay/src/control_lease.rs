/// A control request must finish before the lease it depends on expires.
pub(crate) fn request_deadline(authority: &Authority) -> Instant {
    (Instant::now() + REQUEST_TIMEOUT).min(authority.expires_at)
}

impl Control {
    pub(crate) async fn renew(&mut self, status: &mpsc::Sender<Status>) -> Result<(), RelayError> {
        let authority = self.authority.as_ref().unwrap();
        let sent = Instant::now();
        if !authority.is_current(sent) {
            self.authority = None;
            return self.lost(status).await;
        }
        let grant = authority.grant;
        let deadline = request_deadline(authority);
        let mut client = self.take_call_client().await?;
        let result = client
            .call(self.origin(), Command::Renew { grant }, deadline)
            .await;
        self.return_call_client(client);
        match self.accept(result)? {
            Some(Reply::Lease { grant }) => {
                self.cleanup_grant = Some(grant);
                self.authority = Authority::received(grant, sent, Instant::now());
            }
            None => {}
            _ => return Err(RelayError::InvalidReply),
        }
        if self.authority.is_none() {
            self.lost(status).await?;
        }
        Ok(())
    }

    pub(crate) fn accept(
        &mut self,
        result: Result<Reply, ClientError>,
    ) -> Result<Option<Reply>, RelayError> {
        match result {
            Ok(reply) => Ok(Some(reply)),
            Err(ClientError::Remote(PublicError::Conflict)) => {
                self.authority = None;
                Ok(None)
            }
            Err(error) => {
                self.authority = None;
                Err(error.into())
            }
        }
    }

    pub(crate) async fn lost(&mut self, status: &mpsc::Sender<Status>) -> Result<(), RelayError> {
        publish(status, Status::ControlLost { holder_id: None }).await
    }

    pub async fn release(&mut self) {
        self.authority = None;
        let Some(grant) = self.cleanup_grant.take() else {
            return;
        };
        let deadline = Instant::now() + RELEASE_TIMEOUT;
        let mut released = false;
        if let Some(mut client) = self.client.take().or_else(|| self.lane_client.take()) {
            if client.is_usable() {
                released = client
                    .call(self.origin(), Command::Release { grant }, deadline)
                    .await
                    .is_ok();
            }
        }
        if !released {
            if let Ok(mut client) =
                Client::connect_local(&self.options.socket, self.options.entry, deadline).await
            {
                // Release only the known grant. This is not a retry of uncertain input/acquisition.
                let _ = client
                    .call(self.origin(), Command::Release { grant }, deadline)
                    .await;
            }
        }
    }
}
