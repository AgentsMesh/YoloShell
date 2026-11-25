mod evidence;
mod target;
pub use evidence::{exit_or_tombstone_matches, stable_termination_tombstone_matches};
pub use target::AttachmentTarget;

use anyhow::{Result, ensure};
use yolo_paths::{
    Paths, directory::PrivateDirectory, durable_mutation::SessionDurableMutationGuard,
};

/// Retire this archived session's staged files once its runtime custody is settled.
pub fn clear_staged_attachments_if(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    target: AttachmentTarget,
    settled: impl FnOnce() -> bool,
) -> Result<bool> {
    guard.require_no_pending_intent()?;
    let session = guard
        .namespace()
        .directory(&paths.session_dir(guard.terminal_id()))?;
    ensure!(
        session.object_identity()? == guard.session_identity(),
        "archived session directory changed"
    );
    let path = yolo_paths::attachments::directory(session.path());
    match session.open_descendant(&path) {
        Ok(root) => {
            if !target.matches(&root)? {
                // The previously observed tree was replaced. Its successor has its own contents.
                return Ok(true);
            }
            if !settled() {
                return Ok(false);
            }
            remove_attachment_tree(&session, root)?;
        }
        Err(error) if attachment_errno(&error) == Some(libc::ENOENT) => {
            // A previous attempt may have unlinked the tree before its parent sync failed.
            session.sync()?;
        }
        Err(error) => return Err(error),
    }
    guard.verify_session_directory()?;
    Ok(true)
}

fn remove_attachment_tree(session: &PrivateDirectory, root: PrivateDirectory) -> Result<()> {
    use std::os::unix::ffi::OsStrExt;
    use yolo_paths::directory_stream::DirectoryStream;

    root.verify_path()?;
    let entries = DirectoryStream::open(root.descriptor())?;
    let mut stack = vec![(root, entries)];
    while let Some((directory, entries)) = stack.last_mut() {
        let Some(name) = entries.next_name()? else {
            let (empty, _) = stack.pop().expect("current directory is on the stack");
            let parent = stack.last().map_or(session, |(parent, _)| parent);
            ensure!(
                parent.remove_empty_descendant(&empty)?,
                "staged attachment directory changed"
            );
            continue;
        };
        let path = directory.path().join(&name);
        match directory.open_descendant(&path) {
            Ok(child) => {
                child.verify_path()?;
                let entries = DirectoryStream::open(child.descriptor())?;
                stack.push((child, entries));
            }
            Err(error) if matches!(attachment_errno(&error), Some(libc::ENOTDIR | libc::ELOOP)) => {
                let name = std::ffi::CString::new(name.as_bytes())?;
                // unlinkat never follows the final component. The parent descriptor remains
                // tied to the proven private tree even if its pathname is concurrently moved.
                directory.verify_path()?;
                if unsafe { libc::unlinkat(directory.descriptor(), name.as_ptr(), 0) } != 0 {
                    return Err(std::io::Error::last_os_error().into());
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn attachment_errno(error: &anyhow::Error) -> Option<i32> {
    error
        .downcast_ref::<std::io::Error>()
        .and_then(std::io::Error::raw_os_error)
}
