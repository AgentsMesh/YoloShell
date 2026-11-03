use super::*;
use std::collections::BTreeMap;

#[path = "bash_locale_constants.rs"]
pub(super) mod constants;
use self::constants::*;

pub(super) struct BashPty {
    reader: Option<Box<dyn Read + Send>>,
    fixture: Fixture,
    lease: crate::model::WriterLeaseGrant,
    capture: Vec<u8>,
}

impl BashPty {
    pub(super) async fn start(overrides: BTreeMap<String, String>) -> Self {
        let mut command = CommandBuilder::new(BASH);
        command.env_clear();
        command.args(BASH_ARGS);
        for (key, value) in FIXTURE_ENV {
            command.env(key, value);
        }
        crate::worker::environment::configure(&mut command, &overrides);
        // The existing fixture owns and reaps this exact private PTY child on every exit.
        let fixture = Fixture::with_command(command);
        let lease = fixture
            .state
            .acquire_lease(fixture.controller(), &fixture.lease_request(HOLDER), false)
            .await
            .unwrap();
        let reader = lock_recover(&fixture.state.master)
            .as_ref()
            .unwrap()
            .try_clone_reader()
            .unwrap();
        let mut shell = Self {
            reader: Some(reader),
            fixture,
            lease,
            capture: Vec::new(),
        };
        shell.read_until(PROMPT).await;
        shell
    }

    pub(super) async fn send(&self, bytes: &[u8]) {
        self.fixture
            .state
            .write_input(
                self.fixture.controller(),
                &InputRequest {
                    terminal_id: self.fixture.terminal_id(),
                    runtime_epoch: self.fixture.state.runtime_epoch(),
                    generation: self.lease.generation,
                    writer_capability: self.lease.writer_capability.clone(),
                    input_id: Uuid::new_v4(),
                    data: bytes.to_vec(),
                },
            )
            .await
            .unwrap();
    }

    pub(super) async fn query(&mut self, command: &[u8], marker: &[u8]) -> String {
        self.capture.clear();
        self.send(command).await;
        self.read_until(marker).await;
        // Readline can repaint between UTF-8 bytes; the PTY stream is not a text file.
        String::from_utf8_lossy(&self.capture).into_owned()
    }

    pub(super) async fn screen_contains(&mut self, text: &str) {
        self.wait_for(|shell| {
            lock_recover(&shell.fixture.state.terminal)
                .screen()
                .contents()
                .contains(text)
        })
        .await;
    }

    async fn read_until(&mut self, marker: &[u8]) {
        self.wait_for(|shell| {
            shell
                .capture
                .windows(marker.len())
                .any(|bytes| bytes == marker)
        })
        .await;
    }

    async fn wait_for(&mut self, observed: impl Fn(&Self) -> bool) {
        let deadline = tokio::time::Instant::now() + READ_TIMEOUT;
        while !observed(self) {
            assert!(
                tokio::time::Instant::now() < deadline,
                "{TIMEOUT_DETAIL}: {:?}",
                String::from_utf8_lossy(&self.capture)
            );
            let mut buffer = [0; READ_BYTES];
            match self.reader.as_mut().unwrap().read(&mut buffer) {
                Ok(0) => panic!("{CLOSED_DETAIL}"),
                Ok(count) => {
                    self.capture.extend_from_slice(&buffer[..count]);
                    assert!(self.capture.len() <= CAPTURE_LIMIT, "{CAPTURE_DETAIL}");
                    lock_recover(&self.fixture.state.terminal).process(&buffer[..count]);
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    tokio::time::sleep(READ_POLL).await;
                }
                Err(error) => panic!("{CLOSED_DETAIL}: {error}"),
            }
        }
    }
}

impl Drop for BashPty {
    fn drop(&mut self) {
        // Fixture owns its master/writer; this wrapper owns only its extra reader.
        drop(self.reader.take());
    }
}
