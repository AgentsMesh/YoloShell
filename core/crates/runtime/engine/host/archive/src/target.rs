use anyhow::{Result, ensure};
use std::fs;
use yolo_paths::{
    Paths, directory::PrivateDirectory, durable_mutation::SessionDurableMutationGuard,
    identity::ObjectIdentity,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AttachmentEntry {
    Private(ObjectIdentity),
    Untrusted(ObjectIdentity),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttachmentTarget {
    pub session: ObjectIdentity,
    root: Option<AttachmentEntry>,
}

impl AttachmentTarget {
    pub fn reacquire<'a>(
        self,
        paths: &Paths,
        namespace: &'a yolo_paths::namespace_lease::NamespaceLease,
        id: uuid::Uuid,
    ) -> Result<Option<SessionDurableMutationGuard<'a>>> {
        namespace.verify()?;
        if matches!(fs::symlink_metadata(paths.session_dir(id)), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
        {
            return Ok(None);
        }
        let guard = SessionDurableMutationGuard::acquire(paths, namespace, id)?;
        Ok((guard.session_identity() == self.session).then_some(guard))
    }

    pub fn observe(session: &PrivateDirectory) -> Result<Self> {
        let path = yolo_paths::attachments::directory(session.path());
        let root = match session.open_descendant(&path) {
            Ok(root) => Some(AttachmentEntry::Private(root.object_identity()?)),
            Err(error) if super::attachment_errno(&error) == Some(libc::ENOENT) => None,
            Err(_) => Some(AttachmentEntry::Untrusted(ObjectIdentity::from_metadata(
                &fs::symlink_metadata(path)?,
            ))),
        };
        session.verify_path()?;
        Ok(Self {
            session: session.object_identity()?,
            root,
        })
    }

    pub fn capture(paths: &Paths, guard: &SessionDurableMutationGuard<'_>) -> Result<Self> {
        let parent = guard
            .namespace()
            .directory(&paths.session_dir(guard.terminal_id()))?;
        let target = Self::observe(&parent)?;
        ensure!(
            target.session == guard.session_identity(),
            "archived session directory changed"
        );
        Ok(target)
    }

    pub fn has_entry(self) -> bool {
        self.root.is_some()
    }

    pub(super) fn matches(self, root: &PrivateDirectory) -> Result<bool> {
        match self.root {
            Some(AttachmentEntry::Private(expected)) => Ok(root.object_identity()? == expected),
            Some(AttachmentEntry::Untrusted(expected)) => {
                root.verify_path()?;
                let current = ObjectIdentity::from_metadata(&fs::symlink_metadata(root.path())?);
                root.verify_path()?;
                Ok(current == expected)
            }
            None => Ok(false),
        }
    }
}
