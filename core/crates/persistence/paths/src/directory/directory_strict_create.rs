use super::*;

const CREATE_SYNC_CONTEXT: &str = "sync newly created private directory";
const CREATE_CONTEXT: &str = "create strict private directory";
const EMPTY_ADOPTION_ERROR: &str = "existing private directory is not empty";
const NORMALIZE_SYNC_CONTEXT: &str = "sync normalized private directory";
const PARENT_SYNC_CONTEXT: &str = "sync new private directory parent";
const PATH_CHANGED_ERROR: &str = "private directory path changed";
const ROOT_ERROR: &str = "private directory cannot be root";

impl PrivateDirectory {
    pub fn open_or_create_strict(path: &Path) -> Result<Self> {
        super::super::validation::validate_absolute(path)?;
        let resolved = super::super::platform::platform_path(path);
        let mut current = open_root()?;
        let mut current_path = PathBuf::from(std::path::MAIN_SEPARATOR_STR);
        let components = normal_components(&resolved)?;
        anyhow::ensure!(!components.is_empty(), ROOT_ERROR);
        let component_count = components.len();
        for (index, component) in components.into_iter().enumerate() {
            verify_descriptor_path(&current, &current_path)?;
            let name = entry_c_string(component)?;
            let (next, created) = open_or_create_strict_component(&current, &name)?;
            current_path.push(component);
            if created {
                set_private_mode(&next, &current_path)?;
                next.sync_all().context(CREATE_SYNC_CONTEXT)?;
                current.sync_all().context(PARENT_SYNC_CONTEXT)?;
            }
            let final_component = index + 1 == component_count;
            if final_component {
                if !created {
                    normalize_empty_final_directory(&next, &current_path)?;
                }
                crate::private_acl::require_empty(&next, &current_path)?;
                DirectoryIdentity::inspect_private(&next.metadata()?, &current_path)?;
            } else {
                crate::private_acl::require_no_grants(&next, &current_path)?;
                DirectoryIdentity::inspect_ancestor(&next.metadata()?, &current_path)?;
            }
            current = next;
        }
        let reopened = Self::open(path)?;
        anyhow::ensure!(
            reopened.identity == DirectoryIdentity::inspect_private(&current.metadata()?, path)?,
            "{PATH_CHANGED_ERROR}: {}",
            path.display()
        );
        Ok(reopened)
    }
}

fn normalize_empty_final_directory(file: &File, path: &Path) -> Result<()> {
    if DirectoryIdentity::inspect_private(&file.metadata()?, path).is_ok()
        && crate::private_acl::require_empty(file, path).is_ok()
    {
        return Ok(());
    }
    verify_normalizable_private(file, path)?;
    anyhow::ensure!(directory_is_empty(file)?, EMPTY_ADOPTION_ERROR);
    verify_normalizable_descriptor_path(file, path)?;
    set_private_mode(file, path)?;
    file.sync_all().context(NORMALIZE_SYNC_CONTEXT)?;
    verify_descriptor_path(file, path)?;
    anyhow::ensure!(directory_is_empty(file)?, EMPTY_ADOPTION_ERROR);
    Ok(())
}

fn verify_normalizable_descriptor_path(file: &File, path: &Path) -> Result<()> {
    let expected = DirectoryIdentity::inspect_ancestor(&file.metadata()?, path)?;
    let resolved = super::super::platform::platform_path(path);
    let components = normal_components(&resolved)?;
    let component_count = components.len();
    let mut current = open_root()?;
    let mut current_path = PathBuf::from(std::path::MAIN_SEPARATOR_STR);
    for (index, component) in components.into_iter().enumerate() {
        current = open_directory(current.as_raw_fd(), &entry_c_string(component)?)?;
        current_path.push(component);
        let actual = DirectoryIdentity::inspect_ancestor(&current.metadata()?, &current_path)?;
        if index + 1 == component_count {
            anyhow::ensure!(actual == expected, PATH_CHANGED_ERROR);
            return Ok(());
        }
        crate::private_acl::require_no_grants(&current, &current_path)?;
    }
    anyhow::bail!(ROOT_ERROR)
}

fn directory_is_empty(file: &File) -> Result<bool> {
    let mut stream = super::super::directory_stream::DirectoryStream::open(file.as_raw_fd())?;
    Ok(stream.next_name()?.is_none())
}

fn open_or_create_strict_component(parent: &File, name: &CStr) -> Result<(File, bool)> {
    match open_directory(parent.as_raw_fd(), name) {
        Ok(file) => Ok((file, false)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mode = libc::mode_t::try_from(PRIVATE_DIRECTORY_MODE)?;
            let result = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), mode) };
            let create_error = std::io::Error::last_os_error();
            if result != 0 && create_error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(create_error).context(CREATE_CONTEXT);
            }
            Ok((open_directory(parent.as_raw_fd(), name)?, result == 0))
        }
        Err(error) => Err(error.into()),
    }
}
