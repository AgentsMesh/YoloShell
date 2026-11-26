pub const AUTHORITY: &str = "core/crates/persistence/paths/src/socket/namespace/socket_namespace_authority.rs";
pub const ADMISSION: &str = "core/crates/persistence/paths/src/socket/namespace/socket_namespace_admission.rs";
pub const DIRECTORY: &str = "core/crates/persistence/paths/src/socket/namespace/socket_namespace_directory_lease.rs";
pub const STAGE: &str = "core/crates/persistence/paths/src/socket/namespace/socket_namespace_stage_authority.rs";
pub const BOOKKEEPING: &str =
    "core/crates/persistence/paths/src/socket/namespace/socket_namespace_orphan_bookkeeping.rs";
pub const CERTIFICATE: &str =
    "core/crates/persistence/paths/src/socket/namespace/directory_certificate/constants.rs";
pub const CERTIFICATE_RECORD: &str =
    "core/crates/persistence/paths/src/socket/namespace/directory_certificate/record.rs";
pub const CERTIFICATE_IO: &str =
    "core/crates/persistence/paths/src/socket/namespace/socket_namespace_directory_certificate.rs";
pub const REGISTRATION_READ: &str =
    "core/crates/persistence/paths/src/socket/namespace/registration/read.rs";
pub const REGISTRATION: &str = "core/crates/persistence/paths/src/socket/namespace/socket_namespace_registration.rs";
pub const RELEASE: &str =
    "core/crates/persistence/paths/src/socket/namespace/socket_namespace_registered_cleanup.rs";
pub const REUSED_LEASE_DEADLINE: &str = "Instant::now() + SOCKET_STAGE_LEASE_TIMEOUT";

pub const CONTRACTS: &[(&str, &[&str])] = &[
    (
        AUTHORITY,
        &[
            "directory_lease::DirectoryLease::shared(directory)?",
            "self.directory.take(); self.directory = directory_lease::DirectoryLease::try_exclusive(directory)?;",
            "certificate: directory_certificate::read(directory)?.context(CERTIFICATE_REQUIRED)?",
            "self.registration.verify()?",
            "self.certificate.registration == self.registration.durable_identity()",
            "directory_certificate::verify(directory, &self.certificate)",
        ],
    ),
    (
        ADMISSION,
        &[
            "certificate.binding == location.binding",
            "certified.cleanup_empty(&root, &mut lifecycle)?",
            "authority::Authority::registered(registration, &directory)?",
        ],
    ),
    (
        DIRECTORY,
        &[
            "Self::try_lock(directory, Mode::Shared)",
            "Self::try_lock(directory, Mode::Exclusive)",
            "ProcessFileLock::try_lock(&file, identity, mode)?",
            "BorrowedFd::borrow_raw(directory.descriptor())",
            "directory.object_identity()? == self.identity",
        ],
    ),
    (
        STAGE,
        &[
            "lifecycle.verify_root(root)?; self.verify()?;",
            "directory_lease::DirectoryLease::try_exclusive(&self.directory)?",
            "bookkeeping::settle(&self.directory, || { lifecycle.verify_root(root)?; lease.verify(&self.directory)?; self.verify() })?",
            "if !only_lifecycle_or_empty(&self.directory)? { return Ok(false); }",
            "lease.verify(&self.directory)?; self.verify()?;",
            "empty_remove::owned_locked(root, &self.directory, &name, lifecycle)",
            "live_identity: directory.object_identity()?",
            "self.directory.object_identity()? == self.live_identity",
            "directory_certificate::verify(&self.directory, &self.certificate)",
        ],
    ),
    (
        BOOKKEEPING,
        &[
            "if !only_bookkeeping(directory)? { return Ok(false); }",
            "for _ in 0..SOCKET_STAGE_RETIRED_CHUNK_LIMIT",
            "fence()?; let result = scanner.scan_chunk()?;",
            "fence()?; match result",
        ],
    ),
    (CERTIFICATE, &["const CERTIFICATE_SCHEMA_VERSION: u16 = 3;"]),
    (
        CERTIFICATE_RECORD,
        &[
            "directory: DurableObjectIdentity",
            "registration: DurableObjectIdentity",
            "self.directory == directory",
            "directory.kind() == yolo_durable_identity::ObjectKind::Directory",
            "self.registration.kind() == yolo_durable_identity::ObjectKind::RegularFile",
        ],
    ),
    (
        CERTIFICATE_IO,
        &[
            "certificate.validate(directory.durable_identity()?)?",
            "read_record(directory)?.as_ref() == Some(expected)",
            "Certificate::new(directory.durable_identity()?, expected.clone(), registration)?",
        ],
    ),
    (
        REGISTRATION,
        &[
            "let durable_identity = read::durable_identity(&entry, &opened)?;",
            "self.entry.verify_current_unchanged(self.identity, REGISTRATION_MAX_BYTES)",
        ],
    ),
    (
        REGISTRATION_READ,
        &[
            "record.validate(directory.durable_identity()?, expected)?",
            "opened.durable_version(entry.path(), REGISTRATION_MAX_BYTES)?.object_identity()",
            "entry.verify_current_unchanged(opened.identity(), REGISTRATION_MAX_BYTES)?;",
        ],
    ),
    (
        RELEASE,
        &[
            "namespace.authority.registration.try_upgrade()?",
            "namespace.authority.try_upgrade_directory(&namespace.directory)?",
        ],
    ),
];
