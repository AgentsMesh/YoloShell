use crate::constants::*;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(test)]
#[path = "../unit_tests/workspace_resolution_tests.rs"]
mod resolution_tests;

#[derive(Default)]
struct Locations {
    override_root: Option<OsString>,
    build_root: Option<OsString>,
    test_source: Option<OsString>,
    workspace: Option<OsString>,
    runfiles: Option<OsString>,
    manifest: Option<OsString>,
    executable: Option<PathBuf>,
}

pub fn locate(explicit: Option<&Path>) -> io::Result<PathBuf> {
    if let Some(root) = explicit {
        return directory(root);
    }
    let values = environment_locations();
    locate_with(&values, &env::current_dir()?)
}

pub fn locate_runfiles_overlay() -> io::Result<Option<PathBuf>> {
    runfiles_with(&environment_locations())
}

fn environment_locations() -> Locations {
    Locations {
        override_root: env::var_os(ROOT_OVERRIDE_ENV),
        build_root: env::var_os(BUILD_WORKSPACE_ENV),
        test_source: env::var_os(TEST_SOURCE_DIRECTORY_ENV),
        workspace: env::var_os(TEST_WORKSPACE_ENV),
        runfiles: env::var_os(RUNFILES_DIRECTORY_ENV),
        manifest: env::var_os(RUNFILES_MANIFEST_ENV),
        executable: env::current_exe().ok(),
    }
}

fn locate_with(values: &Locations, current: &Path) -> io::Result<PathBuf> {
    if let Some(root) = &values.override_root {
        return directory(Path::new(root));
    }
    if let Some(root) = joined_workspace(values.test_source.as_ref(), values.workspace.as_deref()) {
        if root.is_dir() {
            return directory(&root);
        }
    }
    let repository = values
        .workspace
        .as_deref()
        .unwrap_or_else(|| OsStr::new(BAZEL_MAIN_REPOSITORY_DIRECTORY));
    let runfiles = values
        .runfiles
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| adjacent_runfiles(values.executable.as_deref()));
    if let Some(root) = runfiles.as_deref().map(|base| base.join(repository)) {
        if root.is_dir() {
            return directory(&root);
        }
    }
    if let Some(manifest) = &values.manifest {
        if let Some(root) = root_from_manifest(Path::new(manifest), repository)? {
            return directory(&root);
        }
    }
    if let Some(root) = &values.build_root {
        return directory(Path::new(root));
    }
    let root = ancestor_workspace(current).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "unable to locate the Bazel workspace",
        )
    })?;
    directory(&root)
}

fn runfiles_with(values: &Locations) -> io::Result<Option<PathBuf>> {
    if let Some(root) = joined_workspace(values.test_source.as_ref(), values.workspace.as_deref()) {
        if root.is_dir() {
            return directory(&root).map(Some);
        }
    }
    let repository = values
        .workspace
        .as_deref()
        .unwrap_or_else(|| OsStr::new(BAZEL_MAIN_REPOSITORY_DIRECTORY));
    let runfiles = values
        .runfiles
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| adjacent_runfiles(values.executable.as_deref()));
    let Some(root) = runfiles.as_deref().map(|base| base.join(repository)) else {
        return Ok(None);
    };
    root.is_dir().then(|| directory(&root)).transpose()
}

fn adjacent_runfiles(executable: Option<&Path>) -> Option<PathBuf> {
    let executable = executable?;
    let name = executable.file_name()?.to_str()?;
    executable
        .parent()
        .map(|parent| parent.join(format!("{name}{BAZEL_RUNFILES_DIRECTORY_SUFFIX}")))
}

fn joined_workspace(base: Option<&OsString>, name: Option<&OsStr>) -> Option<PathBuf> {
    Some(Path::new(base?).join(name?))
}

fn root_from_manifest(path: &Path, workspace: &OsStr) -> io::Result<Option<PathBuf>> {
    let logical = Path::new(workspace).join(WORKSPACE_MARKER);
    let logical = logical.to_string_lossy();
    let contents = fs::read_to_string(path)?;
    for line in contents.lines() {
        let Some((key, value)) = line.split_once(' ') else {
            continue;
        };
        if key == logical {
            return Ok(Path::new(value).parent().map(Path::to_path_buf));
        }
    }
    Ok(None)
}

fn ancestor_workspace(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|candidate| candidate.join(WORKSPACE_MARKER).is_file())
        .map(Path::to_path_buf)
}

fn directory(path: &Path) -> io::Result<PathBuf> {
    if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("workspace directory does not exist: {}", path.display()),
        ));
    }
    path.canonicalize()
}

#[cfg(test)]
pub(crate) fn locate_runfiles(source: &Path, workspace: Option<&str>) -> io::Result<PathBuf> {
    let values = Locations {
        runfiles: Some(source.as_os_str().to_owned()),
        workspace: workspace.map(OsString::from),
        ..Locations::default()
    };
    locate_with(&values, Path::new("/missing"))
}

#[cfg(test)]
pub(crate) fn locate_manifest(path: &Path, workspace: Option<&str>) -> io::Result<PathBuf> {
    let values = Locations {
        manifest: Some(path.as_os_str().to_owned()),
        workspace: workspace.map(OsString::from),
        ..Locations::default()
    };
    locate_with(&values, Path::new("/missing"))
}

#[cfg(test)]
pub(crate) fn locate_executable_runfiles(
    executable: &Path,
    workspace: Option<&str>,
) -> io::Result<PathBuf> {
    let values = Locations {
        executable: Some(executable.to_path_buf()),
        workspace: workspace.map(OsString::from),
        ..Locations::default()
    };
    locate_with(&values, Path::new("/missing"))
}
