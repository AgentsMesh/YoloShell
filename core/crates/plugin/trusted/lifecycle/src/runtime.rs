use std::{
    env,
    path::{Path, PathBuf},
};

pub(crate) fn resolve(program: &str) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.components().count() > 1 {
        return path.is_file().then(|| path.to_owned());
    }
    search()
        .into_iter()
        .map(|directory| directory.join(program))
        .find(|candidate| candidate.is_file())
}

pub(crate) fn search() -> Vec<PathBuf> {
    let mut paths = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        for directory in [home.join(".bun/bin"), home.join(".local/bin")] {
            if !paths.contains(&directory) {
                paths.push(directory);
            }
        }
    }
    for directory in [
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ] {
        if !paths.contains(&directory) {
            paths.push(directory);
        }
    }
    paths
}

pub(crate) fn join(paths: &[PathBuf]) -> String {
    env::join_paths(paths)
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
