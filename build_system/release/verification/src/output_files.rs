use crate::output::OutputDirectory;
use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::{File, Metadata};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{FileExt, MetadataExt};
use yolo_release_contracts::{
    ARCHIVE_NAME, CANDIDATE_EVIDENCE_CHANGED_ERROR, CHECKSUM_NAME, EXPECTED_FILE_LINKS,
    MAX_EVIDENCE_BYTES, PERMISSION_BITS_MASK, PROVENANCE_NAME, READ_ONLY_FILE_MODE,
};
use yolo_release_filesystem::ensure_no_extended_acl;
use yolo_release_gate_system::validate_exact_entries;

pub(crate) struct OpenedCandidate {
    pub artifact: OpenedFile,
    checksum: OpenedFile,
    provenance: OpenedFile,
}

pub(crate) struct OpenedFile {
    pub file: File,
    identity: FileIdentity,
}

impl OutputDirectory {
    pub fn open_artifact(&self) -> Result<File> {
        Ok(OpenedFile::open(self, ARCHIVE_NAME)?.file)
    }

    pub fn read_checksum(&self) -> Result<Vec<u8>> {
        OpenedFile::open(self, CHECKSUM_NAME)?.read_limited()
    }

    pub fn read_provenance(&self) -> Result<Vec<u8>> {
        OpenedFile::open(self, PROVENANCE_NAME)?.read_limited()
    }

    pub(crate) fn open_candidate(&self) -> Result<OpenedCandidate> {
        self.verify_namespace()?;
        validate_exact_entries(&self.directory)?;
        Ok(OpenedCandidate {
            artifact: OpenedFile::open(self, ARCHIVE_NAME)?,
            checksum: OpenedFile::open(self, CHECKSUM_NAME)?,
            provenance: OpenedFile::open(self, PROVENANCE_NAME)?,
        })
    }

    pub(crate) fn verify_candidate(&self, opened: &OpenedCandidate) -> Result<()> {
        opened.artifact.verify(self, ARCHIVE_NAME)?;
        opened.checksum.verify(self, CHECKSUM_NAME)?;
        opened.provenance.verify(self, PROVENANCE_NAME)?;
        validate_exact_entries(&self.directory)?;
        self.verify_namespace()
    }
}

impl OpenedCandidate {
    pub fn checksum(&self) -> Result<Vec<u8>> {
        self.checksum.read_limited()
    }

    pub fn provenance(&self) -> Result<Vec<u8>> {
        self.provenance.read_limited()
    }
}

impl OpenedFile {
    fn open(output: &OutputDirectory, name: &str) -> Result<Self> {
        output.directory.verify_self()?;
        let name = CString::new(name)?;
        let descriptor = unsafe {
            libc::openat(
                output.directory.raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if descriptor < 0 {
            return Err(std::io::Error::last_os_error()).context("open candidate output");
        }
        let file = unsafe { File::from_raw_fd(descriptor) };
        let identity = FileIdentity::capture(&file)?;
        Ok(Self { file, identity })
    }

    fn verify(&self, output: &OutputDirectory, name: &str) -> Result<()> {
        ensure!(
            FileIdentity::capture(&self.file)? == self.identity,
            "opened candidate file changed"
        );
        ensure!(
            Self::open(output, name)?.identity == self.identity,
            "candidate file path identity changed"
        );
        Ok(())
    }

    fn read_limited(&self) -> Result<Vec<u8>> {
        ensure!(
            self.identity.size <= MAX_EVIDENCE_BYTES,
            "candidate evidence is too large"
        );
        let mut bytes = vec![0_u8; usize::try_from(self.identity.size)?];
        let mut offset = 0_usize;
        while offset < bytes.len() {
            let count = self.file.read_at(&mut bytes[offset..], offset as u64)?;
            ensure!(count > 0, CANDIDATE_EVIDENCE_CHANGED_ERROR);
            offset += count;
        }
        ensure!(
            FileIdentity::capture(&self.file)? == self.identity,
            CANDIDATE_EVIDENCE_CHANGED_ERROR
        );
        Ok(bytes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    owner: u32,
    links: u64,
    size: u64,
    change_seconds: i64,
    change_nanoseconds: i64,
}

impl FileIdentity {
    fn capture(file: &File) -> Result<Self> {
        let metadata = file.metadata()?;
        validate_file(file, &metadata)?;
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            owner: metadata.uid(),
            links: metadata.nlink(),
            size: metadata.len(),
            change_seconds: metadata.ctime(),
            change_nanoseconds: metadata.ctime_nsec(),
        })
    }
}

fn validate_file(file: &File, metadata: &Metadata) -> Result<()> {
    ensure!(metadata.is_file(), "candidate output must be a file");
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "candidate output file owner mismatch"
    );
    ensure!(
        metadata.mode() & PERMISSION_BITS_MASK == READ_ONLY_FILE_MODE,
        "candidate output file mode mismatch"
    );
    ensure!(
        metadata.nlink() == EXPECTED_FILE_LINKS,
        "candidate output file must have one link"
    );
    ensure_no_extended_acl(file.as_raw_fd(), "candidate output file")
}
