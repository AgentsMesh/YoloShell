use super::{POLL, constants::*, deadline};
use crate::ssh_transport::SshdFixture;
use std::{
    fs::OpenOptions,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use yolo_hub_ssh_profile_protocol::{SshBrokerLaunch, SshBrokerProfile, SshProfileRef};
use yolo_remote_broker::{BrokerClient, BrokerConfig, BrokerPaths, BrokerStatus};

#[path = "ssh_profile_live_cleanup.rs"]
mod cleanup;
#[path = "ssh_profile_live_records.rs"]
mod records;

pub struct Broker {
    pub registration: SshBrokerProfile,
    paths: BrokerPaths,
    cleanup: cleanup::Custody,
}
impl Broker {
    /// Publishes only configuration; the Hub must start the actual detached broker.
    pub fn prepare(root: &Path, remote_home: &Path, sshd: &SshdFixture) -> Self {
        yolo_runtime::paths::directory::PrivateDirectory::ensure(root).unwrap();
        let known_hosts = root.join(KNOWN_HOSTS_FILE);
        write(
            &known_hosts,
            sshd.known_hosts_text(sshd.address().port()).as_bytes(),
        );
        let executable = crate::runtime_binary().canonicalize().unwrap();
        let endpoint = sshd.address();
        let config_file = root.join(BROKER_CONFIG_FILE);
        let data = serde_json::json!({
            (VERSION_KEY): BROKER_CONFIG_SCHEMA, (HUB_TRANSPORT_KEY): true,
            (HOST_ID_KEY): PROFILE_ID, (AUTHORITY_KEY): AUTHORITY,
            (ENDPOINT_KEY): { (USER_KEY): sshd.user(), (HOST_KEY): endpoint.ip().to_string(), (PORT_KEY): endpoint.port() },
            (KNOWN_HOSTS_KEY): known_hosts, (IDENTITIES_KEY): [sshd.identity().path()], (USE_AGENT_KEY): false,
            (RUNTIME_KEY): { (EXECUTABLE_KEY): executable, (HOME_KEY): remote_home },
            (POLICY_KEY): { (PROGRAMS_KEY): [executable], (READ_ROOTS_KEY): [remote_home] },
        });
        let encoded = serde_json::to_vec(&data).unwrap();
        write(&config_file, &encoded);
        let config = BrokerConfig::load(&config_file).unwrap();
        let paths = BrokerPaths::new(root.join(BROKER_DIRECTORY)).unwrap();
        let registration = SshBrokerProfile {
            launch: Some(SshBrokerLaunch {
                config_source: config_file,
                source_sha256: yolo_crypto::sha256_hex(&encoded),
            }),
            profile: SshProfileRef::new(PROFILE_ID, AUTHORITY).unwrap(),
            broker_state: paths.state_dir().to_owned(),
            config_revision: config.revision().into(),
        };
        let cleanup = cleanup::Custody::new(&paths, &registration);
        assert!(!paths.management_socket().exists());
        Self {
            registration,
            paths,
            cleanup,
        }
    }
    pub fn client(&self) -> BrokerClient {
        BrokerClient::new(self.paths.management_socket())
    }
    pub async fn status(&self) -> BrokerStatus {
        self.client().status().await.unwrap()
    }
    pub async fn capture(&mut self) {
        let mut last_error = None;
        let captured = tokio::time::timeout_at(deadline(), async {
            loop {
                match self.cleanup.capture().await {
                    Ok(status) => return status,
                    Err(error) => {
                        last_error = Some(format!("{error:#}"));
                        tokio::time::sleep(POLL).await;
                    }
                }
            }
        })
        .await;
        assert!(captured.is_ok(), "{INVALID_CUSTODY}: {last_error:?}");
    }
    pub async fn capture_if_ready(&mut self) -> bool {
        self.cleanup.capture().await.is_ok()
    }
    pub fn assert_same_custody(&self) {
        self.cleanup.assert_live();
    }
    pub async fn stop(mut self) {
        self.cleanup.retire().await.unwrap();
        self.cleanup.wait().await;
        assert!(!self.paths.management_socket().exists());
        assert!(yolo_remote_broker::broker_instance_available(&self.paths).unwrap());
    }
}

fn write(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
