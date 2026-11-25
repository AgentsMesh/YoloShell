fn path_is_within(path: &Path, root: &Path) -> bool {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return false;
    }
    match (path.canonicalize(), root.canonicalize()) {
        (Ok(path), Ok(root)) => path.starts_with(root),
        _ => path.is_absolute() && root.is_absolute() && path.starts_with(root),
    }
}
