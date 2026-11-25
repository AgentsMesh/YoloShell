use super::cleanup_io::{Inspection, SocketState};
use super::probe_cleanup::{Hooks, Recovery};
use super::{Kind, Target};
use crate::paths::constants::{HOST_SOCKET_FILE_NAME, PRIVATE_FILE_MODE, SOCKET_ROOT_PREFIX};
use crate::paths::directory::PrivateDirectory;
use std::ffi::{CStr, CString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

const EXPECTED_SOCKET_ALIAS: &str = "expected a private socket alias";
const PROBE_TEMP_PREFIX: &str = "yp-commit-";
const SAVED_ENDPOINT_NAME: &str = "saved-commit-proof-endpoint";
const UNRELATED_SOCKET_NAME: &str = "unrelated-commit-proof-socket";
const UNRELATED_ALIAS_NAME: &str = "unrelated-commit-proof-alias";

#[test]
fn probe_proofs_accept_exact_single_and_double_link_aliases() {
    let fixture = CommitProbeFixture::new();
    let endpoint_name = CString::new(HOST_SOCKET_FILE_NAME).unwrap();
    let (probe_name, _) = fixture.target.stage(Kind::Probe).unwrap();
    let double_link = socket_state(&fixture.directory, &probe_name);

    assert!(
        super::probe_cleanup::proven(
            &fixture.directory,
            &fixture.target,
            Some(&endpoint_name),
            &probe_name,
            double_link,
        )
        .unwrap()
    );
    let unrelated = fixture.directory.path().join(UNRELATED_SOCKET_NAME);
    let unrelated_alias = fixture.directory.path().join(UNRELATED_ALIAS_NAME);
    create_private_socket(&unrelated);
    fs::hard_link(&unrelated, &unrelated_alias).unwrap();
    let unrelated_alias = CString::new(UNRELATED_ALIAS_NAME).unwrap();
    assert!(
        !super::probe_cleanup::proven(
            &fixture.directory,
            &fixture.target,
            Some(&endpoint_name),
            &unrelated_alias,
            double_link,
        )
        .unwrap()
    );

    fs::remove_file(&fixture.endpoint).unwrap();
    fixture.directory.sync().unwrap();
    let single_link = socket_state(&fixture.directory, &probe_name);

    assert!(
        !super::probe_cleanup::proven(
            &fixture.directory,
            &fixture.target,
            Some(&endpoint_name),
            &probe_name,
            single_link,
        )
        .unwrap()
    );
    assert!(
        super::probe_cleanup::alias_proven(
            &fixture.directory,
            &fixture.target,
            Some(&endpoint_name),
            &probe_name,
            single_link,
        )
        .unwrap()
    );
}

#[test]
fn post_unlink_endpoint_replacement_fails_the_final_identity_proof() {
    let fixture = CommitProbeFixture::new();
    fs::rename(&fixture.probe, &fixture.quarantine).unwrap();
    fixture.directory.sync().unwrap();
    let endpoint_name = CString::new(HOST_SOCKET_FILE_NAME).unwrap();
    let (quarantine_name, _) = fixture.target.probe_quarantine().unwrap();
    let expected = socket_state(&fixture.directory, &quarantine_name);
    let saved = fixture.directory.path().join(SAVED_ENDPOINT_NAME);
    let endpoint = fixture.endpoint.clone();
    let mut before_quarantine = || Ok(());
    let mut after_quarantine = || Ok(());
    let mut before_unlink = || Ok(());
    let mut after_unlink = || {
        fs::rename(&endpoint, &saved)?;
        create_private_socket(&endpoint);
        Ok(())
    };
    let mut hooks = Hooks {
        before_quarantine: &mut before_quarantine,
        after_quarantine: &mut after_quarantine,
        before_unlink: &mut before_unlink,
        after_unlink: &mut after_unlink,
    };

    let recovery = super::probe_cleanup::remove_proven(
        &fixture.directory,
        &fixture.target,
        Some(&endpoint_name),
        &quarantine_name,
        expected,
        &mut hooks,
    )
    .unwrap();

    assert_eq!(recovery, Recovery::Unproven);
    assert!(saved.exists());
    assert!(!fixture.quarantine.exists());
}

fn socket_state(directory: &PrivateDirectory, name: &CStr) -> SocketState {
    match super::cleanup_io::inspect(directory, name).unwrap() {
        Inspection::Socket(state) => state,
        Inspection::Missing | Inspection::Unsafe => panic!("{EXPECTED_SOCKET_ALIAS}"),
    }
}

struct CommitProbeFixture {
    _temporary: tempfile::TempDir,
    directory: PrivateDirectory,
    target: Target,
    endpoint: PathBuf,
    probe: PathBuf,
    quarantine: PathBuf,
}

impl CommitProbeFixture {
    fn new() -> Self {
        let parent = Path::new(SOCKET_ROOT_PREFIX).parent().unwrap();
        let temporary = tempfile::Builder::new()
            .prefix(PROBE_TEMP_PREFIX)
            .tempdir_in(parent)
            .unwrap();
        let directory = PrivateDirectory::ensure(temporary.path()).unwrap();
        let endpoint = directory.path().join(HOST_SOCKET_FILE_NAME);
        let endpoint_name = CString::new(HOST_SOCKET_FILE_NAME).unwrap();
        let target = Target::new(directory.path(), &endpoint_name).unwrap();
        let probe = target.stage(Kind::Probe).unwrap().1;
        let quarantine = target.probe_quarantine().unwrap().1;
        create_private_socket(&endpoint);
        fs::hard_link(&endpoint, &probe).unwrap();
        Self {
            _temporary: temporary,
            directory,
            target,
            endpoint,
            probe,
            quarantine,
        }
    }
}

fn create_private_socket(path: &Path) {
    let listener = UnixListener::bind(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    drop(listener);
}
