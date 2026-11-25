const OWNER_CANCELLATION_CONTROL_DIRECTORY_ENV: &str = "YOLO_OWNER_CANCELLATION_CONTROL_DIRECTORY";
const OWNER_CANCELLATION_CONTROL_NONCE_ENV: &str = "YOLO_OWNER_CANCELLATION_CONTROL_NONCE";
const OWNER_CANCELLATION_STARTED_FILE: &str = "started.json";
const OWNER_CANCELLATION_COMPLETED_FILE: &str = "completed.json";
const OWNER_CANCELLATION_ROOT_IDENTITY_FILE: &str = "root.json";
const OWNER_CANCELLATION_INTERMEDIATE_IDENTITY_FILE: &str = "intermediate.json";
const OWNER_CANCELLATION_LEAF_IDENTITY_FILE: &str = "leaf.json";
const OWNER_CANCELLATION_HELPER_SETTLE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum OwnerCancellationStage {
    Started,
    Completed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) enum OwnerCancellationHelperRole {
    Root,
    Intermediate,
    Leaf,
}

const OWNER_CANCELLATION_HELPER_ROLES: &[OwnerCancellationHelperRole] = &[
    OwnerCancellationHelperRole::Root,
    OwnerCancellationHelperRole::Intermediate,
    OwnerCancellationHelperRole::Leaf,
];

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OwnerCancellationHandshake {
    nonce: String,
    stage: OwnerCancellationStage,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OwnerCancellationPublishedIdentity {
    nonce: String,
    role: OwnerCancellationHelperRole,
    pid: u32,
    start_time: u64,
}

#[derive(Clone)]
struct OwnerCancellationContext {
    directory: PathBuf,
    nonce: String,
}

impl OwnerCancellationContext {
    fn from_environment() -> Result<Option<Self>> {
        let Some(directory) = std::env::var_os(OWNER_CANCELLATION_CONTROL_DIRECTORY_ENV) else {
            return Ok(None);
        };
        let nonce = std::env::var(OWNER_CANCELLATION_CONTROL_NONCE_ENV)
            .context("read owner cancellation control nonce")?;
        Uuid::parse_str(&nonce).context("validate owner cancellation control nonce")?;
        let directory = PathBuf::from(directory);
        let valid = directory.is_absolute() && directory.is_dir();
        anyhow::ensure!(valid, "owner cancellation control directory is invalid");
        Ok(Some(Self { directory, nonce }))
    }

    fn required() -> Result<Self> {
        Self::from_environment()?
            .ok_or_else(|| anyhow::anyhow!("owner cancellation control context is missing"))
    }

    fn write_stage(&self, stage: OwnerCancellationStage) -> Result<()> {
        atomic_write_json(
            &self.stage_path(stage),
            &OwnerCancellationHandshake {
                nonce: self.nonce.clone(),
                stage,
            },
        )
    }

    fn require_stage(&self, stage: OwnerCancellationStage) -> Result<()> {
        let document: OwnerCancellationHandshake = read_json(&self.stage_path(stage))?;
        let valid = document.nonce == self.nonce && document.stage == stage;
        anyhow::ensure!(valid, "cancellation handshake content changed");
        Ok(())
    }

    fn publish(
        &self,
        role: OwnerCancellationHelperRole,
        identity: process::ProcessIdentity,
    ) -> Result<()> {
        atomic_write_json(
            &self.identity_path(role),
            &OwnerCancellationPublishedIdentity {
                nonce: self.nonce.clone(),
                role,
                pid: identity.pid,
                start_time: identity.start_time,
            },
        )
    }

    fn read_identity(
        &self,
        role: OwnerCancellationHelperRole,
    ) -> Result<Option<process::ProcessIdentity>> {
        let path = self.identity_path(role);
        if !path.exists() {
            return Ok(None);
        }
        let document: OwnerCancellationPublishedIdentity = read_json(&path)?;
        let nonce_matches = document.nonce == self.nonce;
        anyhow::ensure!(nonce_matches, "published helper nonce changed");
        anyhow::ensure!(document.role == role, "published helper role changed");
        let foreign_pid = document.pid != std::process::id();
        anyhow::ensure!(foreign_pid, "published helper aliases the test process");
        Ok(Some(process::ProcessIdentity {
            pid: document.pid,
            start_time: document.start_time,
        }))
    }

    fn require_identity_inventory(&self) -> Result<()> {
        for &role in OWNER_CANCELLATION_HELPER_ROLES {
            let published = self.read_identity(role)?.is_some();
            anyhow::ensure!(published, "helper identity was not published");
        }
        Ok(())
    }

    fn stage_path(&self, stage: OwnerCancellationStage) -> PathBuf {
        let name = match stage {
            OwnerCancellationStage::Started => OWNER_CANCELLATION_STARTED_FILE,
            OwnerCancellationStage::Completed => OWNER_CANCELLATION_COMPLETED_FILE,
        };
        self.directory.join(name)
    }

    fn identity_path(&self, role: OwnerCancellationHelperRole) -> PathBuf {
        let name = match role {
            OwnerCancellationHelperRole::Root => OWNER_CANCELLATION_ROOT_IDENTITY_FILE,
            OwnerCancellationHelperRole::Intermediate => {
                OWNER_CANCELLATION_INTERMEDIATE_IDENTITY_FILE
            }
            OwnerCancellationHelperRole::Leaf => OWNER_CANCELLATION_LEAF_IDENTITY_FILE,
        };
        self.directory.join(name)
    }
}

fn publish_owner_cancellation_identity(
    role: OwnerCancellationHelperRole,
    identity: process::ProcessIdentity,
) -> Result<()> {
    if let Some(context) = OwnerCancellationContext::from_environment()? {
        context.publish(role, identity)?;
    }
    Ok(())
}

pub(super) fn publish_current_owner_cancellation_identity(role: OwnerCancellationHelperRole) -> Result<()> {
    let identity = process::ProcessIdentity::capture(std::process::id())?;
    publish_owner_cancellation_identity(role, identity)
}
