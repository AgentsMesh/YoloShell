use super::super::constants::*;
use super::super::file::scoped_absolute;
use super::super::{RemoteError, RemoteRuntimeSpec};
use super::{RuntimeActivationRequest, RuntimeArtifact, RuntimeStageRequest};
use std::path::Path;

pub(super) struct DeploymentPlan {
    directory_path: String,
    stable_path: String,
    release_path: String,
    temporary_path: String,
    temporary_link: String,
}

impl DeploymentPlan {
    pub(super) fn new(
        runtime: &RemoteRuntimeSpec,
        artifact: &RuntimeArtifact,
    ) -> Result<Self, RemoteError> {
        let stable_path = runtime.executable_text().to_owned();
        if Path::new(&stable_path)
            .file_name()
            .and_then(|value| value.to_str())
            != Some(RUNTIME_EXECUTABLE_FILE_NAME)
        {
            return Err(RemoteError::InvalidRuntime(
                "deployment target must be the yolo executable".into(),
            ));
        }
        let parent = Path::new(&stable_path)
            .parent()
            .filter(|path| scoped_absolute(path))
            .ok_or_else(|| {
                RemoteError::InvalidRuntime("deployment parent is not safely scoped".into())
            })?;
        let release_name = [
            RUNTIME_RELEASE_PREFIX,
            artifact.platform().path_name(),
            RUNTIME_FILE_SEPARATOR,
            artifact.architecture().path_name(),
            RUNTIME_FILE_SEPARATOR,
            artifact.version(),
            RUNTIME_FILE_SEPARATOR,
            artifact.sha256(),
        ]
        .concat();
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let directory_path = parent.to_string_lossy().into_owned();
        let release_path = parent.join(release_name).to_string_lossy().into_owned();
        let temporary_path = parent
            .join(format!("{RUNTIME_UPLOAD_PREFIX}{nonce}"))
            .to_string_lossy()
            .into_owned();
        let temporary_link = parent
            .join(format!("{RUNTIME_LINK_PREFIX}{nonce}"))
            .to_string_lossy()
            .into_owned();
        if [&release_path, &temporary_path, &temporary_link]
            .iter()
            .any(|path| path.len() > MAX_REMOTE_PATH_BYTES)
        {
            return Err(RemoteError::InvalidRuntime(
                "deployment path exceeds the remote path limit".into(),
            ));
        }
        Ok(Self {
            directory_path,
            stable_path,
            release_path,
            temporary_path,
            temporary_link,
        })
    }

    pub(super) fn stage(&self, artifact: &RuntimeArtifact) -> RuntimeStageRequest {
        RuntimeStageRequest {
            directory_path: self.directory_path.clone(),
            stable_path: self.stable_path.clone(),
            release_path: self.release_path.clone(),
            temporary_path: self.temporary_path.clone(),
            bytes: artifact.shared_bytes(),
            sha256: artifact.sha256().into(),
        }
    }

    pub(super) fn activation(&self, artifact: &RuntimeArtifact) -> RuntimeActivationRequest {
        RuntimeActivationRequest {
            stable_path: self.stable_path.clone(),
            release_path: self.release_path.clone(),
            temporary_link: self.temporary_link.clone(),
            expected_bytes: artifact.bytes().len() as u64,
            sha256: artifact.sha256().into(),
        }
    }

    pub(super) fn release_path(&self) -> &str {
        &self.release_path
    }
}
