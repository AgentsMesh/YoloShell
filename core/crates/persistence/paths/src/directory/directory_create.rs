fn open_or_create(parent: &File, name: &CStr, path: &Path) -> Result<(File, bool)> {
    open_or_create_with_fence(parent, name, path, || Ok(()))
}

fn open_or_create_with_fence<F>(
    parent: &File,
    name: &CStr,
    path: &Path,
    mut commit_fence: F,
) -> Result<(File, bool)>
where
    F: FnMut() -> Result<()>,
{
    match open_directory(parent.as_raw_fd(), name) {
        Ok(file) => Ok((file, false)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            #[cfg(all(test, feature = "yolo-unit-tests"))]
            tests::before_directory_create();
            let mode = libc::mode_t::try_from(PRIVATE_DIRECTORY_MODE)?;
            commit_fence()?;
            let result = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), mode) };
            let create_error = std::io::Error::last_os_error();
            if result != 0 && create_error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(create_error)
                    .with_context(|| format!("create private directory below {}", path.display()));
            }
            let created = result == 0;
            Ok(open_directory(parent.as_raw_fd(), name).map(|file| (file, created))?)
        }
        Err(error) => {
            Err(error).with_context(|| format!("open directory below {}", path.display()))
        }
    }
}
