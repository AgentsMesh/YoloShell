const FIXTURE_HOST_RELEASE_CHANNEL_CAPACITY: usize = 1;
const FIXTURE_HOST_RELEASE_CONTROL_DETAIL: &str =
    "PTY fixture received a mismatched Host release request";
const FIXTURE_HOST_RELEASE_ACK_DETAIL: &str = "PTY fixture could not acknowledge Host release";
const FIXTURE_HOST_RELEASE_COMPLETION_DETAIL: &str = "PTY fixture Host release did not complete";
const FIXTURE_HOST_RELEASE_COMPLETION_TIMEOUT: Duration = Duration::from_secs(12);

struct FixtureHostRelease {
    completion: Option<std::sync::mpsc::Receiver<std::result::Result<(), String>>>,
}

impl FixtureHostRelease {
    fn respond_on(fixture: &Fixture, runtime: &tokio::runtime::Handle) -> Self {
        let _entered = runtime.enter();
        Self::respond(fixture)
    }

    fn respond(fixture: &Fixture) -> Self {
        Self {
            completion: Some(Self::spawn_responder(fixture)),
        }
    }

    fn wait(&mut self) {
        Self::receive(self.completion.take().unwrap()).unwrap();
    }

    async fn finish(mut self) {
        self.finish_result().await.unwrap();
    }

    async fn finish_result(&mut self) -> Result<()> {
        let completion = self.completion.take().unwrap();
        tokio::task::spawn_blocking(move || Self::receive(completion))
            .await
            .context(FIXTURE_HOST_RELEASE_COMPLETION_DETAIL)?
    }

    fn receive(
        completion: std::sync::mpsc::Receiver<std::result::Result<(), String>>,
    ) -> Result<()> {
        let result = completion
            .recv_timeout(FIXTURE_HOST_RELEASE_COMPLETION_TIMEOUT)
            .context(FIXTURE_HOST_RELEASE_COMPLETION_DETAIL)?;
        result.map_err(anyhow::Error::msg)
    }

    fn spawn_responder(
        fixture: &Fixture,
    ) -> std::sync::mpsc::Receiver<std::result::Result<(), String>> {
        let identity = fixture
            .state
            .secondary_custody_authority
            .active_for_test()
            .unwrap();
        let (lease, peer) = test_custody_lease(fixture, identity);
        let expected_offer = lease.offer_for_test();
        fixture
            .state
            .pty_alias_custody
            .install_release_handle(lease.release_handle());
        let hold = tokio::spawn(lease.hold(Arc::clone(&fixture.state)));
        let (sender, receiver) =
            std::sync::mpsc::sync_channel(FIXTURE_HOST_RELEASE_CHANNEL_CAPACITY);
        tokio::spawn(async move {
            let result = respond_to_fixture_release(peer, expected_offer, hold).await;
            let _ = sender.send(result.map_err(|error| error.to_string()));
        });
        receiver
    }
}

async fn respond_to_fixture_release(
    peer: UnixStream,
    expected_offer: crate::pty_custody::CustodyOffer,
    hold: tokio::task::JoinHandle<Result<()>>,
) -> Result<()> {
    let mut peer = crate::pty_custody::CustodyUnixStream::from_local(peer)?;
    let control: crate::pty_custody::CustodyControl = peer.read().await?;
    anyhow::ensure!(
        control.matches_release(&expected_offer),
        FIXTURE_HOST_RELEASE_CONTROL_DETAIL
    );
    let acknowledgement = crate::pty_custody::CustodyControlAck::released(&control)
        .context(FIXTURE_HOST_RELEASE_ACK_DETAIL)?;
    peer.write(&acknowledgement).await?;
    drop(peer);
    hold.await??;
    Ok(())
}
