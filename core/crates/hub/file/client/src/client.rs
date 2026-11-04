use crate::FileClientError;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use yolo_hub_client::Client;
use yolo_hub_file_protocol::{FileRootSummary, MAX_FILE_ROOTS, ReadFileRequest, ReadFileResult};
use yolo_hub_identity::{FileRootEpoch, HubId};
use yolo_hub_rpc_protocol::{Command, Reply};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileRoots {
    pub origin: HubId,
    pub epoch: FileRootEpoch,
    pub roots: Vec<FileRootSummary>,
}

/// Root paths are origin-local and independently authorized; no partial refresh is published.
pub async fn list_roots<S: AsyncRead + AsyncWrite + Unpin>(
    client: &mut Client<S>,
    origin: HubId,
    deadline: Instant,
) -> Result<FileRoots, FileClientError> {
    let mut collected = Vec::new();
    let mut epoch = None;
    let mut after = None;
    loop {
        let Reply::FileRoots {
            epoch: observed,
            roots,
            next,
        } = client
            .call(origin, Command::ListFileRoots { after }, deadline)
            .await?
        else {
            return Err(FileClientError::UnexpectedReply);
        };
        if epoch.is_some_and(|previous| previous != observed) {
            return Err(FileClientError::Changed);
        }
        epoch = Some(observed);
        if roots.len() > MAX_FILE_ROOTS.saturating_sub(collected.len()) {
            return Err(FileClientError::Limit);
        }
        collected.extend(roots);
        match next {
            Some(next) if collected.len() < MAX_FILE_ROOTS => after = Some(next),
            Some(_) => return Err(FileClientError::Limit),
            None => {
                return Ok(FileRoots {
                    origin,
                    epoch: observed,
                    roots: collected,
                });
            }
        }
    }
}

pub async fn read<S: AsyncRead + AsyncWrite + Unpin>(
    client: &mut Client<S>,
    request: ReadFileRequest,
    deadline: Instant,
) -> Result<ReadFileResult, FileClientError> {
    let origin = request.fields().target.address.hub_id;
    match client
        .call(origin, Command::ReadFile { request }, deadline)
        .await?
    {
        Reply::File { file } => Ok(file),
        _ => Err(FileClientError::UnexpectedReply),
    }
}
