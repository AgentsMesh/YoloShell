use crate::delivery_cleanup::remove_stage;
use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use yolo_release_contracts::*;
use yolo_release_contracts::{
    NO_FILE_FLAGS, PRIVATE_DIRECTORY_MODE, RANDOM_DEVICE, RANDOM_SUFFIX_BYTES,
};
use yolo_release_filesystem::{DirectoryHandle, DirectoryResourceOwner};
use yolo_release_foundation::digest::hex_digest;
use yolo_release_gate_system::{ensure_missing, entry_names, validate_directory, validate_parent};

pub struct DeliveryStage {
    parent: DirectoryHandle,
    handle: DirectoryHandle,
    resources: DirectoryResourceOwner,
    name: CString,
    retained: bool,
}

impl DeliveryStage {
    pub fn create(parent: &DirectoryHandle) -> Result<Self> {
        let name = temporary_name()?;
        ensure!(
            unsafe {
                libc::mkdirat(
                    parent.raw_fd(),
                    name.as_ptr(),
                    PRIVATE_DIRECTORY_MODE as libc::mode_t,
                )
            } == DELIVERY_SYSTEM_CALL_SUCCESS,
            DELIVERY_STAGE_ERROR
        );
        let mut handle = DirectoryHandle::open_at_bytes(parent, name.as_bytes())?;
        let resources = DirectoryResourceOwner::attach(&mut handle);
        handle.set_mode(PRIVATE_DIRECTORY_MODE)?;
        validate_directory(
            &handle,
            DELIVERY_STAGE_ERROR,
            PRIVATE_DIRECTORY_MODE,
            NO_FILE_FLAGS,
        )?;
        Ok(Self {
            parent: parent.clone(),
            handle,
            resources,
            name,
            retained: false,
        })
    }

    pub fn handle(&self) -> &DirectoryHandle {
        &self.handle
    }

    pub fn name(&self) -> Result<&str> {
        self.name.to_str().map_err(Into::into)
    }

    pub fn retain(&mut self) {
        self.retained = true;
    }

    pub fn create_extraction(&self) -> Result<DirectoryHandle> {
        let name = CString::new(EXTRACTION_DIRECTORY_NAME)?;
        ensure!(
            unsafe {
                libc::mkdirat(
                    self.handle.raw_fd(),
                    name.as_ptr(),
                    PRIVATE_DIRECTORY_MODE as libc::mode_t,
                )
            } == DELIVERY_SYSTEM_CALL_SUCCESS,
            DELIVERY_EXTRACTION_ERROR
        );
        let extraction = DirectoryHandle::open_at(&self.handle, EXTRACTION_DIRECTORY_NAME)?;
        extraction.set_mode(PRIVATE_DIRECTORY_MODE)?;
        validate_directory(
            &extraction,
            DELIVERY_EXTRACTION_ERROR,
            PRIVATE_DIRECTORY_MODE,
            NO_FILE_FLAGS,
        )?;
        Ok(extraction)
    }

    pub fn install_app(&self, extraction: &DirectoryHandle) -> Result<()> {
        self.handle
            .verify_child(EXTRACTION_DIRECTORY_NAME, extraction)?;
        let app = CString::new(APP_NAME)?;
        ensure!(
            unsafe {
                libc::renameatx_np(
                    extraction.raw_fd(),
                    app.as_ptr(),
                    self.handle.raw_fd(),
                    app.as_ptr(),
                    libc::RENAME_EXCL,
                )
            } == DELIVERY_SYSTEM_CALL_SUCCESS,
            DELIVERY_APP_RENAME_ERROR
        );
        ensure!(
            entry_names(extraction)?.is_empty(),
            DELIVERY_ROOT_ENTRIES_ERROR
        );
        let extraction_name = CString::new(EXTRACTION_DIRECTORY_NAME)?;
        ensure!(
            unsafe {
                libc::unlinkat(
                    self.handle.raw_fd(),
                    extraction_name.as_ptr(),
                    libc::AT_REMOVEDIR,
                )
            } == DELIVERY_SYSTEM_CALL_SUCCESS,
            DELIVERY_APP_RENAME_ERROR
        );
        DirectoryHandle::open_at(&self.handle, APP_NAME).map(|_| ())
    }

    pub fn publish(&mut self, destination: &str) -> Result<()> {
        validate_parent(&self.parent)?;
        ensure_missing(&self.parent, destination)?;
        self.parent
            .verify_child_bytes(self.name.as_bytes(), &self.handle)?;
        let destination = CString::new(destination)?;
        let status = unsafe {
            libc::renameatx_np(
                self.parent.raw_fd(),
                self.name.as_ptr(),
                self.parent.raw_fd(),
                destination.as_ptr(),
                libc::RENAME_EXCL,
            )
        };
        if status != DELIVERY_SYSTEM_CALL_SUCCESS {
            return Err(std::io::Error::last_os_error()).context(DELIVERY_RENAME_ERROR);
        }
        self.retained = true;
        Ok(())
    }
}

impl Drop for DeliveryStage {
    fn drop(&mut self) {
        if !self.retained {
            let cleanup = self
                .resources
                .begin_retirement()
                .and_then(|()| remove_stage(&self.parent, &self.name, &self.handle));
            if let Err(error) = cleanup {
                eprintln!(
                    "{DELIVERY_CLEANUP_ERROR}; retained {}: {error:#}",
                    self.handle.display_path().display()
                );
            }
        }
    }
}

fn temporary_name() -> Result<CString> {
    let mut random = [0_u8; RANDOM_SUFFIX_BYTES];
    File::open(RANDOM_DEVICE)?.read_exact(&mut random)?;
    CString::new(format!("{DELIVERY_TEMPORARY_PREFIX}{}", hex_digest(random))).map_err(Into::into)
}
