use super::*;

const TEST_SOCKET_ROOT_DIRECTORY_NAME: &str = "socket-root";

impl SocketNamespaceLocation {
    pub(super) fn from_digest(
        effective_user_id: u32,
        digest: [u8; SOCKET_FULL_HASH_BYTES],
    ) -> Self {
        let root = std::env::var_os(SOCKET_ROOT_ENVIRONMENT).map(PathBuf::from);
        Self::from_selection(
            effective_user_id,
            digest,
            root_selection::SocketRootSelection::new(effective_user_id, root.as_deref())
                .expect(TEST_SOCKET_ROOT_OVERRIDE_ERROR),
        )
    }

    pub(super) fn from_default_digest(
        effective_user_id: u32,
        digest: [u8; SOCKET_FULL_HASH_BYTES],
    ) -> Self {
        Self::from_selection(
            effective_user_id,
            digest,
            root_selection::SocketRootSelection::default(effective_user_id),
        )
    }
}

pub(super) fn unique_location(test_root: &Path) -> SocketNamespaceLocation {
    isolated_location(test_root, random_digest())
}

fn isolated_location(
    test_root: &Path,
    digest: [u8; SOCKET_FULL_HASH_BYTES],
) -> SocketNamespaceLocation {
    let mut location = SocketNamespaceLocation::from_digest(unsafe { libc::geteuid() }, digest);
    let key = location.directory.file_name().unwrap().to_owned();
    location.root = test_root.join(TEST_SOCKET_ROOT_DIRECTORY_NAME);
    location.directory = location.root.join(key);
    location
}

fn random_digest() -> [u8; SOCKET_FULL_HASH_BYTES] {
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let mut digest = [SOCKET_DIGEST_SEPARATOR; SOCKET_FULL_HASH_BYTES];
    digest[..first.as_bytes().len()].copy_from_slice(first.as_bytes());
    digest[first.as_bytes().len()..].copy_from_slice(second.as_bytes());
    digest
}
