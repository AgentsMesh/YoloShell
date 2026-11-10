use super::*;

const REGISTRATION: &str = r#"
fn retain() {
    namespace.write_json_expected_with_commit_fence(&path, None, &record, || {
        initialization.verify()
    })?;
}
"#;
const ADMISSION: &str = r#"
fn retain() {
    let mut lifecycle = lifecycle_lock::SocketNamespaceLifecycleLock::acquire(&root)?;
    directory_certificate::install_or_verify(&directory, &location.binding, registration.durable_identity())?;
}
"#;
const CERTIFICATE: &str = r#"
pub(super) fn install_or_verify() {
    native_xattr::create(directory.descriptor(), &bytes)?;
    directory.sync()?;
}
"#;
const ATTRIBUTE: &str = r#"
pub(super) fn create() {
    #[cfg(target_os = "macos")]
    libc::fsetxattr(descriptor, ATTRIBUTE_NAME.as_ptr(), bytes.as_ptr().cast(), bytes.len(), ATTRIBUTE_POSITION, libc::XATTR_CREATE);
    #[cfg(target_os = "linux")]
    libc::fsetxattr(descriptor, ATTRIBUTE_NAME.as_ptr(), bytes.as_ptr().cast(), bytes.len(), libc::XATTR_CREATE);
}
"#;
const FUTURE_PATH: &str = "core/crates/persistence/paths/src/socket/namespace/socket_namespace_future.rs";

fn files() -> Vec<SourceFile> {
    [
        (REGISTRATION_PATH, REGISTRATION),
        (ADMISSION_PATH, ADMISSION),
        (CERTIFICATE_PATH, CERTIFICATE),
        (ATTRIBUTE_PATH, ATTRIBUTE),
    ]
    .into_iter()
    .map(|(path, text)| file(path, text))
    .collect()
}

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        absolute: path.into(),
        relative: path.into(),
        text: text.into(),
    }
}

fn inspect_files(files: &[SourceFile]) -> Vec<Violation> {
    let mut violations = Vec::new();
    inspect(files, &mut violations);
    violations
}

#[test]
fn current_publication_requires_registration_fence_lifecycle_lock_and_create_only_certificate() {
    assert!(inspect_files(&files()).is_empty());
}

#[test]
fn removed_or_duplicated_current_producers_fail_the_contract() {
    for path in [
        REGISTRATION_PATH,
        ADMISSION_PATH,
        CERTIFICATE_PATH,
        ATTRIBUTE_PATH,
    ] {
        let mut missing = files();
        missing.retain(|file| file.relative != path);
        assert!(!inspect_files(&missing).is_empty(), "{path}");
        let mut duplicate = files();
        let original = duplicate.iter().find(|file| file.relative == path).unwrap();
        duplicate.push(file(path, &original.text));
        assert!(!inspect_files(&duplicate).is_empty(), "{path}");
    }
}

#[test]
fn lifecycle_fence_no_replace_attestation_and_sync_cannot_be_removed() {
    for (path, original, replacement) in [
        (REGISTRATION_PATH, "initialization.verify()", "Ok(())"),
        (REGISTRATION_PATH, "None", "Some(previous)"),
        (REGISTRATION_PATH, "fn retain", "fn inspect"),
        (
            ADMISSION_PATH,
            "lifecycle_lock::SocketNamespaceLifecycleLock::acquire(&root)?",
            "unprotected_root()",
        ),
        (
            ADMISSION_PATH,
            "registration.durable_identity()",
            "foreign.durable_identity()",
        ),
        (
            ADMISSION_PATH,
            "registration.durable_identity()",
            "registration.identity()",
        ),
        (CERTIFICATE_PATH, "directory.sync()?;", ""),
        (
            CERTIFICATE_PATH,
            "native_xattr::create",
            "native_xattr::replace",
        ),
        (CERTIFICATE_PATH, "pub(super)", "pub(crate)"),
        (ATTRIBUTE_PATH, "pub(super)", "pub"),
        (ATTRIBUTE_PATH, "libc::XATTR_CREATE", "libc::XATTR_REPLACE"),
    ] {
        let mut changed = files();
        let source = changed
            .iter_mut()
            .find(|file| file.relative == path)
            .unwrap();
        source.text = source.text.replace(original, replacement);
        assert!(!inspect_files(&changed).is_empty(), "{path}: {original}");
    }
}

#[test]
fn aliases_nested_functions_and_extra_publication_sites_are_rejected() {
    for source in [
        "use owner::install_or_verify as replace;",
        "use native_xattr::{create as replace};",
        "fn retain() { namespace.write_json_expected_with_commit_fence(&path, None, &record, || { initialization.verify() }); }",
        "fn retain() { directory_certificate::install_or_verify(&directory, &location.binding, registration.durable_identity()); }",
        "fn retain() { native_xattr::create(directory.descriptor(), &bytes); }",
        "fn retain() { namespace.write_json(&path, &record); }",
        "fn retain() { fn nested() { namespace.write_json_expected_with_commit_fence(&path, None, &record, || { initialization.verify() }); } }",
    ] {
        let mut changed = files();
        changed.push(file(FUTURE_PATH, source));
        assert!(!inspect_files(&changed).is_empty(), "{source}");
    }
    let mut changed = files();
    let owner = changed
        .iter_mut()
        .find(|file| file.relative == REGISTRATION_PATH)
        .unwrap();
    owner.text = REGISTRATION.replace("fn retain() {", "fn retain() { fn nested() {") + "}";
    assert!(!inspect_files(&changed).is_empty());
}

#[test]
fn comments_and_strings_cannot_supply_missing_publication_calls() {
    let mut changed = files();
    changed.push(file(
        FUTURE_PATH,
        "// install_or_verify()\nconst NOTE: &str = \"write_json()\";",
    ));
    assert!(inspect_files(&changed).is_empty());
    let owner = changed
        .iter_mut()
        .find(|file| file.relative == REGISTRATION_PATH)
        .unwrap();
    owner.text = format!("/* {REGISTRATION} */");
    assert!(!inspect_files(&changed).is_empty());
}
