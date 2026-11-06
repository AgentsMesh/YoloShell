use super::super::factory::{
    FactoryBackend, LiveFactoryBackend, RUNTIME_INSTALLATION_LOSS_CONTEXT, RusshConnectionFactory,
};
use super::super::*;
use super::config_tests::ConfigFixture;
use super::support::*;
use crate::remote::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const MACHO_ARM64: &[u8] = &[0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1];
const ARTIFACT_VERSION: &str = "1.0.0";
const IDENTITY_PATH: &str = "/private/identity";
const AGENT_PATH: &str = "/private/agent.sock";
const INSTALL_TRANSPORT_FAILURE: &str = "upload interrupted";
const INSTALLATION_FAILURE_REQUIRED: &str = "runtime installation unexpectedly succeeded";

struct RecordingBackend {
    connection: Arc<dyn MultiplexedSshConnection>,
    connect_failure: bool,
    install_failure: bool,
    connects: AtomicUsize,
    installs: AtomicUsize,
    identity_digests: Mutex<Vec<Vec<Option<String>>>>,
}

impl RecordingBackend {
    fn new(connect_failure: bool, install_failure: bool) -> Arc<Self> {
        Self::with_connection(FakeConnection::new([]), connect_failure, install_failure)
    }

    fn with_connection<C>(
        connection: Arc<C>,
        connect_failure: bool,
        install_failure: bool,
    ) -> Arc<Self>
    where
        C: MultiplexedSshConnection + 'static,
    {
        Arc::new(Self {
            connection,
            connect_failure,
            install_failure,
            connects: AtomicUsize::new(0),
            installs: AtomicUsize::new(0),
            identity_digests: Mutex::new(Vec::new()),
        })
    }
}

impl FactoryBackend for RecordingBackend {
    fn connect(
        &self,
        config: SshConnectConfig,
    ) -> RemoteFuture<'_, Arc<dyn MultiplexedSshConnection>> {
        Box::pin(async move {
            self.connects.fetch_add(1, Ordering::Relaxed);
            let digests = config
                .identities_for_broker_test()
                .iter()
                .map(|identity| {
                    identity
                        .material_for_broker_test()
                        .map(yolo_crypto::sha256_hex)
                })
                .collect();
            self.identity_digests.lock().unwrap().push(digests);
            if self.connect_failure {
                return Err(RemoteError::AuthenticationFailed("denied".into()));
            }
            Ok(Arc::clone(&self.connection))
        })
    }

    fn install<'a>(
        &'a self,
        _: &'a dyn MultiplexedSshConnection,
        _: &'a RemoteRuntimeSpec,
        _: &'a RuntimeArtifact,
    ) -> RemoteFuture<'a, ()> {
        Box::pin(async move {
            self.installs.fetch_add(1, Ordering::Relaxed);
            if self.install_failure {
                return Err(RemoteError::transport(INSTALL_TRANSPORT_FAILURE));
            }
            Ok(())
        })
    }
}

fn artifact() -> RuntimeArtifact {
    RuntimeArtifact::new(
        ARTIFACT_VERSION,
        RemoteArchitecture::Aarch64,
        MACHO_ARM64,
        yolo_crypto::sha256_hex(MACHO_ARM64),
    )
    .unwrap()
}

#[tokio::test]
async fn factory_connects_without_optional_credentials_or_install() {
    let backend = RecordingBackend::new(false, false);
    let factory = RusshConnectionFactory::with_backend(test_config(), backend.clone());
    factory.connect().await.unwrap();
    assert_eq!(backend.connects.load(Ordering::Relaxed), 1);
    assert_eq!(backend.installs.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn factory_applies_credentials_and_installs_artifact() {
    let backend = RecordingBackend::new(false, false);
    let mut config = test_config();
    config
        .identities
        .push(IdentityFile::new(IDENTITY_PATH, None));
    config.agent_socket = Some(AGENT_PATH.into());
    config.artifact = Some(artifact());
    let factory = RusshConnectionFactory::with_backend(config, backend.clone());
    factory.connect().await.unwrap();
    assert_eq!(backend.connects.load(Ordering::Relaxed), 1);
    assert_eq!(backend.installs.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn reconnect_keeps_loaded_identity_material_after_path_rewrite() {
    let fixture = ConfigFixture::new();
    let initial_material = std::fs::read(&fixture.identity).unwrap();
    let config = BrokerConfig::load(fixture.write(&fixture.value())).unwrap();
    let backend = RecordingBackend::new(false, false);
    let factory = RusshConnectionFactory::with_backend(config, backend.clone());

    factory.connect().await.unwrap();
    fixture.rotate_identity();
    let replacement_material = std::fs::read(&fixture.identity).unwrap();
    factory.connect().await.unwrap();

    let initial_digest = yolo_crypto::sha256_hex(&initial_material);
    let replacement_digest = yolo_crypto::sha256_hex(&replacement_material);
    assert_ne!(initial_digest, replacement_digest);
    assert_eq!(
        backend.identity_digests.lock().unwrap().as_slice(),
        [
            vec![Some(initial_digest.clone())],
            vec![Some(initial_digest)]
        ]
    );
}

#[tokio::test]
async fn connect_and_install_failures_propagate_without_false_connection() {
    let backend = RecordingBackend::new(true, false);
    let factory = RusshConnectionFactory::with_backend(test_config(), backend.clone());
    assert!(matches!(
        factory.connect().await,
        Err(RemoteError::AuthenticationFailed(_))
    ));
    assert_eq!(backend.installs.load(Ordering::Relaxed), 0);

    let backend = RecordingBackend::new(false, true);
    let mut config = test_config();
    config.artifact = Some(artifact());
    let factory = RusshConnectionFactory::with_backend(config, backend.clone());
    let Err(error) = factory.connect().await else {
        panic!("{INSTALLATION_FAILURE_REQUIRED}")
    };
    assert!(matches!(
        error,
        RemoteError::RuntimeDeployment(message)
            if message.starts_with(RUNTIME_INSTALLATION_LOSS_CONTEXT)
    ));
    assert_eq!(backend.installs.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn live_backend_reports_tcp_failure_and_runs_installer_contract() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut config = test_config();
    config.endpoint = RemoteEndpoint::new("builder", "127.0.0.1", port).unwrap();
    assert!(RusshConnectionFactory::new(config).connect().await.is_err());

    let config = test_config();
    let connection = FakeConnection::new([]);
    LiveFactoryBackend
        .install(connection.as_ref(), config.runtime(), &artifact())
        .await
        .unwrap();
}
