use super::{constants::*, fake::Fake};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use yolo_broker_hub_protocol::BrokerHubProfile;
use yolo_distribution_bundle::{ArtifactRole, BIN_DIRECTORY, BundleSource, MANIFEST_FILE_NAME};
use yolo_paths::directory::PrivateDirectory;
use yolo_remote_provisioning::RemoteProvisioningService;
use yolo_remote_provisioning_protocol::{
    PROVISION_SCHEMA_VERSION, ProvisionFields, ProvisionRequest,
};

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub parent: PathBuf,
    pub request: ProvisionRequest,
    pub remote: Fake,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().canonicalize().unwrap();
        let stage = yolo_remote_bundle_test_support::fixtures::stage();
        let source = base.join(SOURCE);
        fs::create_dir_all(source.join(BIN_DIRECTORY)).unwrap();
        fs::write(
            source.join(MANIFEST_FILE_NAME),
            serde_json::to_vec(stage.descriptor()).unwrap(),
        )
        .unwrap();
        fs::set_permissions(
            source.join(MANIFEST_FILE_NAME),
            fs::Permissions::from_mode(FILE_MODE),
        )
        .unwrap();
        for role in ArtifactRole::ALL {
            let path = source.join(role.relative_path());
            fs::write(&path, stage.payload(role)).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_MODE)).unwrap();
        }
        let profile = BrokerHubProfile {
            host_id: PROFILE.into(),
            profile_authority_sha256: DIGEST.into(),
            config_revision: DIGEST.into(),
        };
        let mut bootstrap = yolo_hub_remote_startup_test_support::request()
            .fields()
            .clone();
        bootstrap.bundle = stage.descriptor().clone();
        let request = ProvisionRequest::new(ProvisionFields {
            schema_version: PROVISION_SCHEMA_VERSION,
            request_id: bootstrap.request_id,
            profile,
            source: BundleSource::new(source, stage.descriptor().clone()).unwrap(),
            install_root: stage.install_root().into(),
            state: STATE.into(),
            host_home: HOME.into(),
            bootstrap: yolo_remote_provisioning_protocol::RemoteBootstrapRequest::new(bootstrap)
                .unwrap(),
        })
        .unwrap();
        Self {
            parent: base.join(BROKER),
            root,
            request,
            remote: Fake::new(),
        }
    }
    pub fn service(&self) -> RemoteProvisioningService {
        RemoteProvisioningService::open(
            PrivateDirectory::ensure(&self.parent).unwrap(),
            self.request.fields().profile.clone(),
        )
        .unwrap()
    }
    pub fn file(&self) -> PathBuf {
        self.parent
            .join(yolo_remote_provisioning::PROVISIONING_DIRECTORY)
            .join(self.request.fields().request_id.to_string())
            .join(RECORD_FILE)
    }
}
pub fn authorized() -> anyhow::Result<()> {
    Ok(())
}
