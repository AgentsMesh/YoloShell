const NATIVE_HOME_ANCHOR: &str = "native Codex hook home";
const NATIVE_SESSION_ANCHOR: &str = "native Codex hook session";

fn retain_native_namespace(
    namespace: &WorkerNamespace,
    paths: &crate::paths::Paths,
    terminal_id: Uuid,
) -> Result<Arc<crate::namespace_lease::NamespaceLease>> {
    let lease = crate::namespace_lease::NamespaceLease::from_directories(vec![
        (
            NATIVE_HOME_ANCHOR,
            namespace.lease().directory(paths.home())?,
        ),
        (
            NATIVE_SESSION_ANCHOR,
            namespace
                .lease()
                .directory(&paths.session_dir(terminal_id))?,
        ),
    ])?;
    namespace.verify()?;
    Ok(Arc::new(lease))
}
