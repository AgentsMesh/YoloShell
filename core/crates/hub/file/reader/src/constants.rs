pub(crate) const DIRECTORY_FLAGS: libc::c_int =
    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK;
pub(crate) const FILE_FLAGS: libc::c_int =
    libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK;
pub(crate) const ROOT_PATH: &str = "/";
pub(crate) const PATH_SEPARATOR: char = '/';
pub(crate) const SINGLE_LINK: u64 = 1;
pub(crate) const MAX_PROTECTED_ROOTS: usize = 16;
pub(crate) const FILE_VERSION_DOMAIN: &str = "yolo:file-read:v1";
pub(crate) const FILE_ROOT_IDENTITY_DOMAIN: &str = "yolo:file-root:v1";
