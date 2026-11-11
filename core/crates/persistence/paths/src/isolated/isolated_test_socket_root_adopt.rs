use super::*;
use anyhow::{Context, Result};
use std::os::unix::ffi::OsStrExt;

const ADOPT_PATH_DETAIL: &str = "isolated test socket root adoption path is invalid";
const ADOPT_CONTENT_DETAIL: &str = "isolated test socket root is not empty during adoption";

pub(super) fn adopt(path: &Path) -> Result<IsolatedTestSocketRoot> {
    let parent_path = Path::new(ISOLATED_TEST_SOCKET_PARENT);
    anyhow::ensure!(path.parent() == Some(parent_path), ADOPT_PATH_DETAIL);
    let name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!(ADOPT_PATH_DETAIL))?;
    validate_name(name.as_bytes())?;
    let name = CString::new(name.as_bytes())?;
    let (parent, parent_identity) = create::open_parent_at(parent_path)?;
    let opened = create::open_child(&parent, &name)?;
    crate::private_acl::require_empty(&opened, path)?;
    let root_identity = ObjectIdentity::from_file(&opened)?
        .context("isolated socket root birth identity is unavailable")?;
    let root = PrivateDirectory::from_opened(opened, path)?;
    require_empty(&root)?;
    let mut adopted = IsolatedTestSocketRoot {
        parent,
        parent_identity,
        root,
        root_identity,
        active_name: name.clone(),
        active_path: path.to_path_buf(),
        name,
        path: path.to_path_buf(),
        stage: IsolatedTestSocketRootStage::Active,
        removal_synced: false,
        finalization_attempted: false,
    };
    hooks::adopt_before_verify(path)?;
    create::verify(&mut adopted)?;
    Ok(adopted)
}

fn validate_name(name: &[u8]) -> Result<()> {
    let token = name.strip_prefix(ISOLATED_TEST_SOCKET_PREFIX.as_bytes());
    anyhow::ensure!(
        token.is_some_and(|token| {
            token.len() == ISOLATED_TEST_SOCKET_TOKEN_HEX_LENGTH
                && token
                    .iter()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        }),
        ADOPT_PATH_DETAIL
    );
    Ok(())
}

fn require_empty(root: &PrivateDirectory) -> Result<()> {
    let mut entries = super::super::directory_stream::DirectoryStream::open(root.descriptor())?;
    anyhow::ensure!(entries.next_name()?.is_none(), ADOPT_CONTENT_DETAIL);
    Ok(())
}
