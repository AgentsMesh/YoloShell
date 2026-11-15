use super::{CodexAdmissionAuthority, CodexHookArtifacts};
use anyhow::{Context, Result, ensure};
use uuid::Uuid;

#[path = "admission_scan_empty_invocation.rs"]
mod empty_invocation;

const SCAN_CORRUPT: &str = "Codex admission artifact scan is corrupt";
const ENTRY_INCREMENT: usize = 1;

#[cfg(any(test, feature = "runtime-component-tests"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/admission_scan_hook.rs"]
mod test_hook;

pub fn scan_admissions(
    paths: &crate::paths::Paths,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
) -> Result<Vec<CodexAdmissionAuthority>> {
    let root_path = CodexHookArtifacts::runtime_directory(paths, terminal_id, runtime_epoch)?;
    let root =
        crate::paths::directory::PrivateDirectory::ensure(&root_path).context(SCAN_CORRUPT)?;
    empty_invocation::reclaim(&root, paths, terminal_id, runtime_epoch)?;
    let revision = root.revision().context(SCAN_CORRUPT)?;
    let mut stream = crate::paths::directory_stream::DirectoryStream::open(root.descriptor())
        .context(SCAN_CORRUPT)?;
    let mut authorities = Vec::new();
    let mut entries_seen = usize::default();
    while let Some(name) = stream.next_name().context(SCAN_CORRUPT)? {
        entries_seen = entries_seen
            .checked_add(ENTRY_INCREMENT)
            .context(SCAN_CORRUPT)?;
        ensure!(
            entries_seen <= crate::paths::MAX_PRIVATE_DIRECTORY_ENTRIES,
            SCAN_CORRUPT
        );
        let invocation_id = name
            .to_str()
            .and_then(|name| Uuid::parse_str(name).ok())
            .context(SCAN_CORRUPT)?;
        let artifacts = CodexHookArtifacts::new(paths, terminal_id, runtime_epoch, invocation_id)?;
        ensure!(
            artifacts.directory() == root.path().join(&name),
            SCAN_CORRUPT
        );
        match CodexAdmissionAuthority::recover_artifacts(&artifacts)? {
            Some(authority) => authorities.push(authority),
            None => ensure!(
                super::admission_orphan::is_pre_admission_orphan(&artifacts)?,
                SCAN_CORRUPT
            ),
        }
    }
    drop(stream);
    #[cfg(any(test, feature = "runtime-component-tests"))]
    test_hook::run();
    root.verify_path().context(SCAN_CORRUPT)?;
    ensure!(
        root.revision().context(SCAN_CORRUPT)? == revision,
        SCAN_CORRUPT
    );
    authorities.sort_by_key(|authority| authority.record().route_revision());
    ensure!(
        authorities
            .windows(2)
            .all(|pair| { pair[0].record().route_revision() < pair[1].record().route_revision() }),
        SCAN_CORRUPT
    );
    for authority in &authorities {
        authority.verify().context(SCAN_CORRUPT)?;
    }
    root.verify_path().context(SCAN_CORRUPT)?;
    Ok(authorities)
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub fn set_before_validation_hook(hook: impl FnOnce() + 'static) {
    test_hook::set(hook);
}
