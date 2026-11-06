use crate::delivery_files::ProtectedFile;
use crate::delivery_receipt_record::DeliveryReceiptRecord;
use crate::delivery_stage::DeliveryStage;
use crate::delivery_target::DeliveryTarget;
use anyhow::{Result, anyhow, ensure};
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::FileExt;
use std::sync::Arc;
use yolo_release_contracts::*;
use yolo_release_contracts::{RANDOM_DEVICE, RANDOM_SUFFIX_BYTES};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_foundation::digest::{bytes_sha256, hex_digest};
use yolo_release_foundation::json::decode;
use yolo_release_gate_system::{ensure_missing, entry_exists, validate_parent};
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

pub struct DeliveryReceipt {
    file: Arc<ProtectedFile>,
    name: String,
    record: DeliveryReceiptRecord,
}

pub struct StagedDeliveryReceipt {
    parent: DirectoryHandle,
    file: Option<ProtectedFile>,
    temporary_name: CString,
    final_name: String,
    record: DeliveryReceiptRecord,
    published: bool,
}

impl DeliveryReceipt {
    pub fn open(target: &DeliveryTarget) -> Result<Option<Self>> {
        let name = receipt_name(&target.destination);
        if !entry_exists(&target.parent, &name)? {
            return Ok(None);
        }
        let file = ProtectedFile::open(&target.parent, &name)?;
        let bytes = read_bounded(file.file())?;
        let record: DeliveryReceiptRecord = decode(&bytes)?;
        ensure!(record.bytes()? == bytes, DELIVERY_RECEIPT_ERROR);
        let receipt = Self {
            file: Arc::new(file),
            name,
            record,
        };
        receipt.verify(target)?;
        Ok(Some(receipt))
    }

    pub fn verify(&self, target: &DeliveryTarget) -> Result<()> {
        self.file
            .verify_contents(&target.parent, &self.name, &self.record.bytes()?)
    }

    pub fn validate(
        &self,
        target: &DeliveryTarget,
        candidate: &VerifiedDeliveryCandidate,
    ) -> Result<()> {
        self.verify(target)?;
        self.record.validate(target, candidate)
    }

    pub fn record(&self) -> &DeliveryReceiptRecord {
        &self.record
    }
}

impl StagedDeliveryReceipt {
    pub fn create(target: &DeliveryTarget, record: DeliveryReceiptRecord) -> Result<Self> {
        let final_name = receipt_name(&target.destination);
        ensure_missing(&target.parent, &final_name)?;
        let temporary_name = temporary_name()?;
        let bytes = record.bytes()?;
        let temporary = temporary_name.to_str()?;
        let file = ProtectedFile::create(&target.parent, temporary, |file| {
            file.write_all(&bytes).map_err(Into::into)
        })?;
        Ok(Self {
            parent: target.parent.clone(),
            file: Some(file),
            temporary_name,
            final_name,
            record,
            published: false,
        })
    }

    pub fn publish(mut self, stage: &mut DeliveryStage) -> Result<DeliveryReceipt> {
        validate_parent(&self.parent)?;
        ensure_missing(&self.parent, &self.final_name)?;
        let final_name = CString::new(self.final_name.as_str())?;
        ensure!(
            unsafe {
                libc::renameatx_np(
                    self.parent.raw_fd(),
                    self.temporary_name.as_ptr(),
                    self.parent.raw_fd(),
                    final_name.as_ptr(),
                    libc::RENAME_EXCL,
                )
            } == DELIVERY_SYSTEM_CALL_SUCCESS,
            DELIVERY_RECEIPT_ERROR
        );
        self.published = true;
        stage.retain();
        let bytes = self.record.bytes()?;
        let file = self
            .file
            .take()
            .ok_or_else(|| anyhow!(DELIVERY_RECEIPT_ERROR))?
            .bind_after_rename(&self.parent, &self.final_name, &bytes)?;
        Ok(DeliveryReceipt {
            file: Arc::new(file),
            name: self.final_name.clone(),
            record: self.record.clone(),
        })
    }
}

impl Drop for StagedDeliveryReceipt {
    fn drop(&mut self) {
        if self.published {
            return;
        }
        let Ok(name) = self.temporary_name.to_str() else {
            return;
        };
        let Some(file) = self.file.as_ref() else {
            return;
        };
        if file.verify_at(&self.parent, name).is_ok() {
            unsafe {
                libc::unlinkat(
                    self.parent.raw_fd(),
                    self.temporary_name.as_ptr(),
                    DELIVERY_UNLINK_FILE_FLAGS,
                );
            }
        }
    }
}

pub fn receipt_name(destination: &str) -> String {
    format!(
        "{DELIVERY_RECEIPT_PREFIX}{}",
        bytes_sha256(destination.as_bytes())
    )
}

fn temporary_name() -> Result<CString> {
    let mut random = [0_u8; RANDOM_SUFFIX_BYTES];
    File::open(RANDOM_DEVICE)?.read_exact(&mut random)?;
    CString::new(format!(
        "{DELIVERY_RECEIPT_TEMPORARY_PREFIX}{}",
        hex_digest(random)
    ))
    .map_err(Into::into)
}

fn read_bounded(file: &File) -> Result<Vec<u8>> {
    let length = file.metadata()?.len();
    ensure!(length <= MAX_DELIVERY_RECEIPT_BYTES, DELIVERY_RECEIPT_ERROR);
    let mut bytes = vec![0_u8; usize::try_from(length)?];
    file.read_exact_at(&mut bytes, 0)?;
    Ok(bytes)
}
