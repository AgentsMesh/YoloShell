use super::*;

pub(crate) struct FederationHost {
    harness: Harness,
    stop: tokio::sync::watch::Sender<bool>,
    keep_alive: tokio::task::JoinHandle<()>,
}
impl std::ops::Deref for FederationHost {
    type Target = Harness;
    fn deref(&self) -> &Harness {
        &self.harness
    }
}
impl Drop for FederationHost {
    fn drop(&mut self) {
        self.stop.send_replace(true);
        self.keep_alive.abort();
    }
}

pub(crate) async fn federation_harness() -> FederationHost {
    let home = tempdir().unwrap();
    Paths::new(home.path()).ensure_base().unwrap();
    let bin = home.path().join(AGENT_BIN_DIR);
    std::fs::create_dir_all(&bin).unwrap();
    install_fake_agent(CODEX_PROVIDER, &bin.join(CODEX_PROVIDER.executable));
    let host = spawn_agent_host(home.path(), CODEX_PROVIDER, &bin);
    let harness = Harness::start_with_host(home, host).await;
    let client = harness.client();
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let keep_alive = tokio::spawn(async move {
        client.hold_host(stopped).await;
    });
    FederationHost {
        harness,
        stop,
        keep_alive,
    }
}

pub(crate) fn federation_agent_command(home: &Path) -> Vec<u8> {
    matrix::agent_start_command(CODEX_PROVIDER, &home.join(CODEX_DATA_HOME_DIRECTORY), home)
}
