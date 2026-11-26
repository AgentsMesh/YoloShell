use super::super::CodexHookArtifacts;
use anyhow::{Context, Result, ensure};
use uuid::Uuid;

const RECLAIM_CORRUPT: &str = "Codex empty invocation reclaim is corrupt";
const RECLAIM_BUDGET_MULTIPLIER: usize = 16;
const MAX_RECLAIMED_INVOCATIONS: usize =
    crate::paths::MAX_PRIVATE_DIRECTORY_ENTRIES * RECLAIM_BUDGET_MULTIPLIER;
const RECLAIM_INCREMENT: usize = 1;

pub(super) fn reclaim(
    root: &crate::paths::directory::PrivateDirectory,
    paths: &crate::paths::Paths,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
) -> Result<()> {
    let mut reclaimed = usize::default();
    loop {
        let mut removed = false;
        let mut stream = crate::paths::directory_stream::DirectoryStream::open(root.descriptor())
            .context(RECLAIM_CORRUPT)?;
        while let Some(name) = stream.next_name().context(RECLAIM_CORRUPT)? {
            if reclaimed == MAX_RECLAIMED_INVOCATIONS {
                return Ok(());
            }
            let Some(invocation_id) = name.to_str().and_then(|value| Uuid::parse_str(value).ok())
            else {
                continue;
            };
            let artifacts =
                CodexHookArtifacts::new(paths, terminal_id, runtime_epoch, invocation_id)?;
            ensure!(
                artifacts.directory() == root.path().join(&name),
                RECLAIM_CORRUPT
            );
            let invocation = root
                .open_descendant(artifacts.directory())
                .context(RECLAIM_CORRUPT)?;
            if root.remove_empty_descendant(&invocation)? {
                reclaimed = reclaimed
                    .checked_add(RECLAIM_INCREMENT)
                    .context(RECLAIM_CORRUPT)?;
                removed = true;
            }
        }
        drop(stream);
        root.verify_path().context(RECLAIM_CORRUPT)?;
        if !removed {
            return Ok(());
        }
    }
}
