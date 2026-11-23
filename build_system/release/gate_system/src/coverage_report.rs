use crate::coverage_report_io::{CoverageDirectory, create_file, reopen_file};
use crate::output_namespace::validate_named_entries;
use crate::output_validation::validate_directory;
use crate::requirements::coverage_report_path;
use crate::snapshot::GateSnapshot;
use anyhow::{Context, Result, ensure};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::sync::Arc;
use yolo_release_bazel_output::BazelOutputBinding;
use yolo_release_contracts::{
    BAZEL_OUT_LINK_NAME, EXPECTED_FILE_LINKS, MAX_RUST_COVERAGE_REPORT_BYTES, PERMISSION_BITS_MASK,
    READ_ONLY_FILE_MODE, RUST_COVERAGE_CHANGED_ERROR, RUST_COVERAGE_CUSTODY_FILE_NAMES,
    RUST_COVERAGE_DESCRIPTION, RUST_COVERAGE_NAMESPACE_ERROR, RUST_COVERAGE_REPORT_RELATIVE_PATH,
    RUST_COVERAGE_SIZE_ERROR, RUST_COVERAGE_TYPE_ERROR, RUST_COVERAGE_UNSEALED_ERROR,
    SEALED_DIRECTORY_MODE,
};
use yolo_release_filesystem::DirectorySeal;
use yolo_release_filesystem::ensure_no_extended_acl;
use yolo_release_foundation::digest::{copy_opened_bounded, opened_file_sha256};
use yolo_release_foundation::file_seal::FileSeal;

const SYSCALL_SUCCESS: libc::c_int = 0;

pub(crate) struct PinnedCoverageReport {
    custody: CoverageDirectory,
    root_seal: Option<DirectorySeal>,
    file: Arc<File>,
    file_seal: FileSeal,
    sha256: String,
}

impl PinnedCoverageReport {
    pub fn capture(snapshot: &GateSnapshot) -> Result<Self> {
        Self::capture_with(snapshot, || Ok(()))
    }

    fn capture_with<F>(snapshot: &GateSnapshot, after_copy: F) -> Result<Self>
    where
        F: FnOnce() -> Result<()>,
    {
        let binding = BazelOutputBinding::capture(
            &snapshot.output,
            &snapshot.links,
            BAZEL_OUT_LINK_NAME,
            RUST_COVERAGE_REPORT_RELATIVE_PATH,
        )?;
        let report = Self::copy_from(snapshot, binding.file())?;
        after_copy()?;
        binding.verify()?;
        report.ensure_unchanged()?;
        Ok(report)
    }

    fn copy_from(snapshot: &GateSnapshot, source: &File) -> Result<Self> {
        let metadata = source.metadata()?;
        ensure!(metadata.is_file(), RUST_COVERAGE_TYPE_ERROR);
        ensure!(
            metadata.len() <= MAX_RUST_COVERAGE_REPORT_BYTES,
            RUST_COVERAGE_SIZE_ERROR
        );
        let source_seal = FileSeal::capture(source)?;
        let source_sha256 = opened_file_sha256(source)?;
        let custody = CoverageDirectory::create(snapshot.temporary())?;
        let mut writable = create_file(custody.handle())?;
        copy_opened_bounded(
            source,
            &mut writable,
            MAX_RUST_COVERAGE_REPORT_BYTES,
            RUST_COVERAGE_DESCRIPTION,
        )?;
        source_seal.verify(source, RUST_COVERAGE_CHANGED_ERROR)?;
        writable.sync_all()?;
        let status =
            unsafe { libc::fchmod(writable.as_raw_fd(), READ_ONLY_FILE_MODE as libc::mode_t) };
        ensure!(status == SYSCALL_SUCCESS, RUST_COVERAGE_CHANGED_ERROR);
        writable.sync_all()?;
        drop(writable);
        let file = Arc::new(reopen_file(custody.handle())?);
        validate_file(&file)?;
        let file_seal = FileSeal::capture(&file)?;
        let sha256 = opened_file_sha256(&file)?;
        ensure!(
            opened_file_sha256(source)? == source_sha256,
            RUST_COVERAGE_CHANGED_ERROR
        );
        source_seal.verify(source, RUST_COVERAGE_CHANGED_ERROR)?;
        ensure!(sha256 == source_sha256, RUST_COVERAGE_CHANGED_ERROR);
        let mut report = Self {
            custody,
            root_seal: None,
            file,
            file_seal,
            sha256,
        };
        report.seal()?;
        report.ensure_unchanged()?;
        ensure!(
            report.path() == coverage_report_path(&snapshot.execution()?),
            RUST_COVERAGE_CHANGED_ERROR
        );
        Ok(report)
    }

    fn seal(&mut self) -> Result<()> {
        validate_named_entries(
            self.custody.handle(),
            RUST_COVERAGE_CUSTODY_FILE_NAMES,
            RUST_COVERAGE_NAMESPACE_ERROR,
        )?;
        self.custody.handle().set_mode(SEALED_DIRECTORY_MODE)?;
        self.custody.handle().set_flags(libc::UF_IMMUTABLE)?;
        self.custody.handle().sync_all()?;
        validate_directory(
            self.custody.handle(),
            RUST_COVERAGE_DESCRIPTION,
            SEALED_DIRECTORY_MODE,
            libc::UF_IMMUTABLE,
        )?;
        self.root_seal = Some(self.custody.handle().seal()?);
        Ok(())
    }

    pub fn ensure_unchanged(&self) -> Result<()> {
        let seal = self
            .root_seal
            .as_ref()
            .context(RUST_COVERAGE_UNSEALED_ERROR)?;
        self.custody.handle().verify_seal(seal)?;
        validate_directory(
            self.custody.handle(),
            RUST_COVERAGE_DESCRIPTION,
            SEALED_DIRECTORY_MODE,
            libc::UF_IMMUTABLE,
        )?;
        self.file_seal
            .verify(&self.file, RUST_COVERAGE_CHANGED_ERROR)?;
        self.file_seal.verify(
            &reopen_file(self.custody.handle())?,
            RUST_COVERAGE_CHANGED_ERROR,
        )?;
        validate_named_entries(
            self.custody.handle(),
            RUST_COVERAGE_CUSTODY_FILE_NAMES,
            RUST_COVERAGE_NAMESPACE_ERROR,
        )?;
        ensure!(
            opened_file_sha256(&self.file)? == self.sha256,
            RUST_COVERAGE_CHANGED_ERROR
        );
        Ok(())
    }

    pub fn path(&self) -> &Path {
        self.custody.path()
    }

    #[cfg(test)]
    pub fn capture_with_hook<F>(snapshot: &GateSnapshot, hook: F) -> Result<Self>
    where
        F: FnOnce() -> Result<()>,
    {
        Self::capture_with(snapshot, hook)
    }

    #[cfg(test)]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

fn validate_file(file: &File) -> Result<()> {
    let metadata = file.metadata()?;
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        RUST_COVERAGE_CHANGED_ERROR
    );
    ensure!(
        metadata.permissions().mode() & PERMISSION_BITS_MASK == READ_ONLY_FILE_MODE,
        RUST_COVERAGE_CHANGED_ERROR
    );
    ensure!(
        metadata.nlink() == EXPECTED_FILE_LINKS,
        RUST_COVERAGE_CHANGED_ERROR
    );
    ensure_no_extended_acl(file.as_raw_fd(), RUST_COVERAGE_DESCRIPTION)
}
