use crate::link::LinkSnapshot;
use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::{self, File};
use std::os::fd::FromRawFd;
use std::path::{Component, Path, PathBuf};
use yolo_release_contracts::{
    BAZEL_OUTPUT_COMPONENT_ERROR, BAZEL_OUTPUT_CONTAINMENT_ERROR, BAZEL_OUTPUT_FILE_CHANGED_ERROR,
    BAZEL_OUTPUT_FILE_NAME_ERROR, BAZEL_OUTPUT_FILE_OPEN_ERROR, BAZEL_OUTPUT_LINK_CHANGED_ERROR,
    BAZEL_OUTPUT_PATH_ERROR, BAZEL_OUTPUT_RESOLUTION_ERROR, BAZEL_OUTPUT_ROOT_CHANGED_ERROR,
};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_foundation::file_seal::FileSeal;

pub struct BazelOutputBinding {
    output: DirectoryHandle,
    links: DirectoryHandle,
    link_name: String,
    relative_suffix: PathBuf,
    link: LinkSnapshot,
    canonical_root: PathBuf,
    canonical_file: PathBuf,
    file: File,
    file_seal: FileSeal,
}

impl BazelOutputBinding {
    pub fn capture(
        output: &DirectoryHandle,
        links: &DirectoryHandle,
        link_name: &str,
        relative_suffix: &str,
    ) -> Result<Self> {
        output.verify_path()?;
        links.verify_path()?;
        let canonical_root =
            fs::canonicalize(output.display_path()).context(BAZEL_OUTPUT_RESOLUTION_ERROR)?;
        let link = LinkSnapshot::capture(links, link_name)?;
        let suffix = normalized_components(Path::new(relative_suffix))?;
        let canonical_file = resolve(&canonical_root, links, &link.target, &suffix)?;
        let file = open_canonical(output, &canonical_root, &canonical_file)?;
        let file_seal = FileSeal::capture(&file)?;
        let binding = Self {
            output: output.clone(),
            links: links.clone(),
            link_name: link_name.to_owned(),
            relative_suffix: PathBuf::from(relative_suffix),
            link,
            canonical_root,
            canonical_file,
            file,
            file_seal,
        };
        binding.verify()?;
        Ok(binding)
    }

    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn verify(&self) -> Result<()> {
        self.output.verify_path()?;
        self.links.verify_path()?;
        ensure!(
            fs::canonicalize(self.output.display_path())? == self.canonical_root,
            BAZEL_OUTPUT_ROOT_CHANGED_ERROR
        );
        ensure!(
            LinkSnapshot::capture(&self.links, &self.link_name)? == self.link,
            BAZEL_OUTPUT_LINK_CHANGED_ERROR
        );
        let suffix = normalized_components(&self.relative_suffix)?;
        ensure!(
            resolve(
                &self.canonical_root,
                &self.links,
                &self.link.target,
                &suffix
            )? == self.canonical_file,
            BAZEL_OUTPUT_LINK_CHANGED_ERROR
        );
        self.file_seal
            .verify(&self.file, BAZEL_OUTPUT_FILE_CHANGED_ERROR)?;
        let reopened = open_canonical(&self.output, &self.canonical_root, &self.canonical_file)?;
        self.file_seal
            .verify(&reopened, BAZEL_OUTPUT_FILE_CHANGED_ERROR)
    }
}

fn resolve(
    canonical_root: &Path,
    links: &DirectoryHandle,
    target: &Path,
    suffix: &[String],
) -> Result<PathBuf> {
    let base = if target.is_absolute() {
        target.to_owned()
    } else {
        links.display_path().join(target)
    };
    let candidate = suffix
        .iter()
        .fold(base, |path, component| path.join(component));
    let canonical = fs::canonicalize(candidate).context(BAZEL_OUTPUT_RESOLUTION_ERROR)?;
    let relative = canonical
        .strip_prefix(canonical_root)
        .context(BAZEL_OUTPUT_CONTAINMENT_ERROR)?;
    normalized_components(relative)?;
    Ok(canonical)
}

fn open_canonical(
    output: &DirectoryHandle,
    canonical_root: &Path,
    canonical: &Path,
) -> Result<File> {
    let relative = canonical
        .strip_prefix(canonical_root)
        .context(BAZEL_OUTPUT_CONTAINMENT_ERROR)?;
    let components = normalized_components(relative)?;
    let (name, parents) = components
        .split_last()
        .context(BAZEL_OUTPUT_FILE_NAME_ERROR)?;
    let mut directory = output.clone();
    for component in parents {
        directory = DirectoryHandle::open_at(&directory, component)?;
    }
    let name = CString::new(name.as_str())?;
    let descriptor = unsafe {
        libc::openat(
            directory.raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context(BAZEL_OUTPUT_FILE_OPEN_ERROR);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn normalized_components(path: &Path) -> Result<Vec<String>> {
    path.components()
        .map(|component| match component {
            Component::Normal(name) => Ok(name
                .to_str()
                .context(BAZEL_OUTPUT_COMPONENT_ERROR)?
                .to_owned()),
            _ => anyhow::bail!(BAZEL_OUTPUT_PATH_ERROR),
        })
        .collect()
}
