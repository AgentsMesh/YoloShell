impl Control {
    pub(crate) async fn take_call_client(&mut self) -> Result<LocalClient, RelayError> {
        if !self.input_busy {
            if let Some(client) = self.client.take().or_else(|| self.lane_client.take()) {
                return Ok(client);
            }
        } else if let Some(client) = self.lane_client.take() {
            return Ok(client);
        }
        Ok(Client::connect_local(
            &self.options.socket,
            self.options.entry,
            Instant::now() + REQUEST_TIMEOUT,
        )
        .await?)
    }

    pub(crate) fn return_call_client(&mut self, client: LocalClient) {
        if !client.is_usable() {
            return;
        }
        if self.input_busy {
            if self.lane_client.is_none() {
                self.lane_client = Some(client);
            }
        } else if self.client.is_none() {
            self.client = Some(client);
        }
    }
}
