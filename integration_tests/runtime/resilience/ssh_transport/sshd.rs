use super::constants::*;
use crate::support::ChildGuard;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;
use yolo_remote::{IdentityFile, KnownHosts};
use yolo_runtime::paths::SOCKET_ROOT_ENVIRONMENT;

#[path = "sshd_keys.rs"]
mod keys;
use keys::{generate_key, public_key_fields, public_key_path};

#[path = "socket_root.rs"]
mod socket_root;

pub(crate) struct SshdFixture {
    _process: ChildGuard,
    _state: TempDir,
    address: SocketAddr,
    user: String,
    identity: PathBuf,
    host_key: String,
    log: PathBuf,
    remote_home: PathBuf,
    readable_file: PathBuf,
}

impl SshdFixture {
    pub(crate) async fn start() -> Self {
        assert!(
            Path::new(SSHD_PROGRAM).is_file(),
            "OpenSSH sshd is required"
        );
        let state = tempfile::tempdir().unwrap();
        let root = state.path().canonicalize().unwrap();
        yolo_runtime::paths::directory::PrivateDirectory::ensure(&root).unwrap();
        let host_key = root.join(HOST_KEY_FILE);
        let identity = root.join(USER_KEY_FILE);
        generate_key(&host_key);
        generate_key(&identity);
        let public_host_key = public_key_fields(&host_key);
        let public_identity = public_key_path(&identity);
        let remote_home = root.join(REMOTE_HOME_DIRECTORY);
        std::fs::create_dir(&remote_home).unwrap();
        let remote_socket_root = socket_root::inherited();
        let readable_file = root.join(FILE_FIXTURE_NAME);
        std::fs::write(&readable_file, FILE_FIXTURE_CONTENT).unwrap();
        let port = unused_port();
        let log_path = root.join(SSHD_LOG_FILE);
        let log = std::fs::File::create(&log_path).unwrap();
        let pid_option = format!(
            "{SSHD_PID_OPTION_PREFIX}{}",
            root.join(SSHD_PID_FILE).display()
        );
        let keys_option = format!("{SSHD_KEYS_OPTION_PREFIX}{}", public_identity.display());
        let environment_option = format!(
            "{SSHD_SET_ENV_OPTION_PREFIX}{SOCKET_ROOT_ENVIRONMENT}={}",
            remote_socket_root.display()
        );
        let process = Command::new(SSHD_PROGRAM)
            .args([
                SSHD_FOREGROUND_ARGUMENT,
                SSHD_STDERR_ARGUMENT,
                SSHD_PORT_ARGUMENT,
            ])
            .arg(port.to_string())
            .arg(SSHD_HOST_KEY_ARGUMENT)
            .arg(&host_key)
            .args([
                SSHD_OPTION_ARGUMENT,
                SSHD_STRICT_MODES_OPTION,
                SSHD_OPTION_ARGUMENT,
                SSHD_PASSWORD_OPTION,
                SSHD_OPTION_ARGUMENT,
                SSHD_INTERACTIVE_OPTION,
                SSHD_OPTION_ARGUMENT,
                SSHD_PAM_OPTION,
                SSHD_OPTION_ARGUMENT,
                SSHD_SFTP_OPTION,
                SSHD_OPTION_ARGUMENT,
                SSHD_FORWARDING_OPTION,
                SSHD_OPTION_ARGUMENT,
                SSHD_STREAMLOCAL_FORWARDING_OPTION,
                SSHD_OPTION_ARGUMENT,
                &pid_option,
                SSHD_OPTION_ARGUMENT,
                &keys_option,
                SSHD_OPTION_ARGUMENT,
                &environment_option,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        wait_until_ready(address).await;
        Self {
            _process: ChildGuard::new(process),
            _state: state,
            address,
            user: current_user(),
            identity,
            host_key: public_host_key,
            log: log_path,
            remote_home,
            readable_file,
        }
    }

    pub(crate) fn address(&self) -> SocketAddr {
        self.address
    }

    pub(crate) fn user(&self) -> &str {
        &self.user
    }

    pub(crate) fn identity(&self) -> IdentityFile {
        IdentityFile::new(self.identity.clone(), None)
    }

    pub(crate) fn identity_path(&self) -> &Path {
        &self.identity
    }

    pub(crate) fn public_host_key(&self) -> &str {
        &self.host_key
    }

    pub(crate) fn known_hosts(&self, port: u16) -> KnownHosts {
        KnownHosts::parse(&self.known_hosts_text(port)).unwrap()
    }
    pub(crate) fn known_hosts_text(&self, port: u16) -> String {
        format!("[{LOOPBACK_HOST}]:{port} {}", self.host_key)
    }

    pub(super) fn remote_home(&self) -> &Path {
        &self.remote_home
    }

    pub(super) fn readable_file(&self) -> &Path {
        &self.readable_file
    }

    pub(crate) fn diagnostics(&self) -> String {
        std::fs::read_to_string(&self.log)
            .unwrap_or_else(|error| format!("sshd log unavailable: {error}"))
    }
}

fn current_user() -> String {
    let output = Command::new(USER_ID_PROGRAM)
        .arg(USER_NAME_ARGUMENT)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn unused_port() -> u16 {
    TcpListener::bind((LOOPBACK_HOST, 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

async fn wait_until_ready(address: SocketAddr) {
    for _ in 0..SSH_READY_ATTEMPTS {
        if tokio::net::TcpStream::connect(address).await.is_ok() {
            return;
        }
        tokio::time::sleep(SSH_READY_INTERVAL).await;
    }
    panic!("isolated sshd did not become ready at {address}");
}
