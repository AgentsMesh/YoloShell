pub(super) const REGISTRATION_PATH: &str =
    "core/crates/persistence/paths/src/socket/namespace/socket_namespace_registration.rs";
pub(super) const ADMISSION_PATH: &str =
    "core/crates/persistence/paths/src/socket/namespace/socket_namespace_admission.rs";
pub(super) const CERTIFICATE_PATH: &str =
    "core/crates/persistence/paths/src/socket/namespace/socket_namespace_directory_certificate.rs";
pub(super) const ATTRIBUTE_PATH: &str =
    "core/crates/persistence/paths/src/socket/namespace/directory_certificate/native_xattr.rs";
pub(super) const RETAIN: &str = "retain";
pub(super) const INSTALL: &str = stringify!(install_or_verify);
pub(super) const CREATE: &str = stringify!(create);
const SINGLE_PUBLICATION: usize = 1;
const SUPPORTED_PLATFORM_COUNT: usize = 2;

pub(super) struct Contract {
    pub path: &'static str,
    pub function: &'static str,
    pub callee: &'static str,
    pub calls: &'static [&'static str],
    pub count: usize,
}

pub(super) const CONTRACTS: &[Contract] = &[
    Contract {
        path: REGISTRATION_PATH,
        function: RETAIN,
        callee: "write_json_expected_with_commit_fence",
        calls: &[
            "namespace.write_json_expected_with_commit_fence(&path, None, &record, || { initialization.verify() })",
        ],
        count: SINGLE_PUBLICATION,
    },
    Contract {
        path: ADMISSION_PATH,
        function: RETAIN,
        callee: INSTALL,
        calls: &[
            "directory_certificate::install_or_verify(&directory, &location.binding, registration.durable_identity())",
        ],
        count: SINGLE_PUBLICATION,
    },
    Contract {
        path: CERTIFICATE_PATH,
        function: INSTALL,
        callee: CREATE,
        calls: &["native_xattr::create(directory.descriptor(), &bytes)"],
        count: SINGLE_PUBLICATION,
    },
    Contract {
        path: ATTRIBUTE_PATH,
        function: CREATE,
        callee: "fsetxattr",
        calls: &[
            "libc::fsetxattr(descriptor, ATTRIBUTE_NAME.as_ptr(), bytes.as_ptr().cast(), bytes.len(), ATTRIBUTE_POSITION, libc::XATTR_CREATE)",
            "libc::fsetxattr(descriptor, ATTRIBUTE_NAME.as_ptr(), bytes.as_ptr().cast(), bytes.len(), libc::XATTR_CREATE)",
        ],
        count: SUPPORTED_PLATFORM_COUNT,
    },
    Contract {
        path: ADMISSION_PATH,
        function: RETAIN,
        callee: "acquire",
        calls: &["lifecycle_lock::SocketNamespaceLifecycleLock::acquire(&root)"],
        count: SINGLE_PUBLICATION,
    },
    Contract {
        path: CERTIFICATE_PATH,
        function: INSTALL,
        callee: "sync",
        calls: &["directory.sync()"],
        count: SINGLE_PUBLICATION,
    },
];

pub(super) const BROAD_WRITERS: &[&str] = &[
    "write_json",
    "write_json_expected",
    "write_json_expected_with_commit_observer",
    "atomic_write_entry_guarded_with_commit_fence",
];
