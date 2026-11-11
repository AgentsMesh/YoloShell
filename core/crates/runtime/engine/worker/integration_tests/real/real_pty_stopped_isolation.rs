const STOPPED_OUTPUT_CONTROL_ENV: &str = "YOLO_REAL_PTY_STOPPED_CONTROL";
const STOPPED_OUTPUT_NONCE_ENV: &str = "YOLO_REAL_PTY_STOPPED_NONCE";
const STOPPED_OUTPUT_ISOLATED_IDENTITY_FILE: &str = "isolated.json";
const STOPPED_OUTPUT_ROOT_IDENTITY_FILE: &str = "root.json";
const STOPPED_OUTPUT_COMPLETED_FILE: &str = "completed.json";
const STOPPED_OUTPUT_CHILD_TIMEOUT_SECONDS: u64 = 60;
const STOPPED_OUTPUT_CHILD_TIMEOUT: Duration =
    Duration::from_secs(STOPPED_OUTPUT_CHILD_TIMEOUT_SECONDS);
const STOPPED_OUTPUT_CHILD_REAP_TIMEOUT: Duration = Duration::from_secs(3);
const STOPPED_OUTPUT_CHILD_TIMEOUT_DETAIL: &str = "isolated stopped-output case timed out";
const STOPPED_OUTPUT_CHILD_REAP_DETAIL: &str = "isolated stopped-output case was not reaped";
const STOPPED_OUTPUT_CHILD_KILL_DETAIL: &str = "terminate isolated stopped-output direct child";
const STOPPED_OUTPUT_CHILD_FAILURE_DETAIL: &str = "isolated stopped-output case failed";
const STOPPED_OUTPUT_CONTEXT_DETAIL: &str = "stopped-output control context is missing";
const STOPPED_OUTPUT_CONTEXT_INVALID_DETAIL: &str = "stopped-output control context is invalid";
const STOPPED_OUTPUT_IDENTITY_CHANGED_DETAIL: &str = "stopped-output identity evidence changed";
const STOPPED_OUTPUT_SESSION_INVALID_DETAIL: &str = "stopped-output child is not session isolated";
const STOPPED_OUTPUT_ROOT_LIVE_DETAIL: &str = "stopped-output PTY root remained live";
const STOPPED_OUTPUT_FAILURE_SEPARATOR: &str = "; ";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum StoppedOutputIdentityRole {
    Isolated,
    Root,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoppedOutputIdentityDocument {
    nonce: String,
    case: String,
    role: StoppedOutputIdentityRole,
    pid: u32,
    start_time: u64,
    unique_id: u64,
}

struct StoppedOutputPublishedIdentity {
    durable: process::ProcessIdentity,
    live: process::LiveProcessIdentity,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoppedOutputCompletionDocument {
    nonce: String,
    case: String,
}

#[derive(Clone)]
struct StoppedOutputContext {
    directory: PathBuf,
    nonce: String,
    case: StoppedOutputCase,
}

impl StoppedOutputContext {
    fn required() -> Result<Self> {
        let directory = std::env::var_os(STOPPED_OUTPUT_CONTROL_ENV)
            .map(PathBuf::from)
            .context(STOPPED_OUTPUT_CONTEXT_DETAIL)?;
        let nonce = std::env::var(STOPPED_OUTPUT_NONCE_ENV)?;
        Uuid::parse_str(&nonce)?;
        let case = StoppedOutputCase::parse(&std::env::var(STOPPED_OUTPUT_CASE_ENV)?)?;
        anyhow::ensure!(
            directory.is_absolute() && directory.is_dir(),
            STOPPED_OUTPUT_CONTEXT_INVALID_DETAIL
        );
        Ok(Self {
            directory,
            nonce,
            case,
        })
    }

    fn publish_isolated_identity(&self) -> Result<()> {
        self.publish(
            StoppedOutputIdentityRole::Isolated,
            process::ProcessIdentity::capture(std::process::id())?,
        )
    }

    fn publish_root(&self, identity: process::ProcessIdentity) -> Result<()> {
        self.publish(StoppedOutputIdentityRole::Root, identity)
    }

    fn publish(
        &self,
        role: StoppedOutputIdentityRole,
        identity: process::ProcessIdentity,
    ) -> Result<()> {
        let metadata = process::kernel_process_metadata_for_test(identity.pid)?;
        anyhow::ensure!(
            metadata.start_time == identity.start_time,
            STOPPED_OUTPUT_IDENTITY_CHANGED_DETAIL
        );
        let live = process::LiveProcessIdentity::from_macos_unique_id_for_test(
            identity.pid,
            metadata.unique_id,
        )?;
        atomic_write_json(
            &self.identity_path(role),
            &StoppedOutputIdentityDocument {
                nonce: self.nonce.clone(),
                case: self.case.name().to_owned(),
                role,
                pid: identity.pid,
                start_time: identity.start_time,
                unique_id: live.macos_unique_id_for_test(),
            },
        )
    }

    fn read_identity(
        &self,
        role: StoppedOutputIdentityRole,
    ) -> Result<Option<StoppedOutputPublishedIdentity>> {
        let path = self.identity_path(role);
        if !path.exists() {
            return Ok(None);
        }
        let document: StoppedOutputIdentityDocument = read_json(&path)?;
        let valid = document.nonce == self.nonce
            && document.case == self.case.name()
            && document.role == role
            && document.pid != std::process::id()
            && document.start_time != u64::default();
        anyhow::ensure!(valid, STOPPED_OUTPUT_IDENTITY_CHANGED_DETAIL);
        let live = process::LiveProcessIdentity::from_macos_unique_id_for_test(
            document.pid,
            document.unique_id,
        )?;
        Ok(Some(StoppedOutputPublishedIdentity {
            durable: process::ProcessIdentity {
                pid: document.pid,
                start_time: document.start_time,
            },
            live,
        }))
    }

    fn complete(&self) -> Result<()> {
        atomic_write_json(
            &self.directory.join(STOPPED_OUTPUT_COMPLETED_FILE),
            &StoppedOutputCompletionDocument {
                nonce: self.nonce.clone(),
                case: self.case.name().to_owned(),
            },
        )
    }

    fn settle_root(&self) -> Result<()> {
        let Some(root) = self.read_identity(StoppedOutputIdentityRole::Root)? else {
            return Ok(());
        };
        force_test_process_tree(root.durable, &root.live)
    }

    fn assert_root_gone(&self) -> Result<()> {
        let root = self
            .read_identity(StoppedOutputIdentityRole::Root)?
            .context(STOPPED_OUTPUT_CONTEXT_DETAIL)?;
        anyhow::ensure!(!root.live.try_matches()?, STOPPED_OUTPUT_ROOT_LIVE_DETAIL);
        Ok(())
    }

    fn identity_path(&self, role: StoppedOutputIdentityRole) -> PathBuf {
        self.directory.join(match role {
            StoppedOutputIdentityRole::Isolated => STOPPED_OUTPUT_ISOLATED_IDENTITY_FILE,
            StoppedOutputIdentityRole::Root => STOPPED_OUTPUT_ROOT_IDENTITY_FILE,
        })
    }
}

include!("real_pty_stopped_verification.rs");
include!("../../test_support/integration/real_pty/real_pty_stopped_watchdog.rs");
include!("../../test_support/integration/real_pty/real_pty_stopped_runner.rs");
