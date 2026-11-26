use crate::constants::*;
use crate::metadata::{stable, validate_descriptor};
use anyhow::{Context, Result, ensure};
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use std::fs::OpenOptions;
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub(crate) struct OpenReceiptFile {
    path: PathBuf,
    file: std::fs::File,
    metadata: std::fs::Metadata,
    bytes: Vec<u8>,
}

impl OpenReceiptFile {
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn verify_unchanged(&self) -> Result<()> {
        let current = self.file.metadata().context(RECEIPT_FILE_ERROR)?;
        ensure!(stable(&self.metadata, &current), RECEIPT_FILE_ERROR);
        validate_descriptor(&self.file, &current)?;
        let current_bytes = read_from_file(&self.file, self.metadata.len())?;
        ensure!(current_bytes == self.bytes, RECEIPT_FILE_ERROR);
        let current_after_read = self.file.metadata().context(RECEIPT_FILE_ERROR)?;
        ensure!(
            stable(&self.metadata, &current_after_read),
            RECEIPT_FILE_ERROR
        );
        validate_descriptor(&self.file, &current_after_read)?;
        let reopened = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(&self.path)
            .context(RECEIPT_FILE_ERROR)?;
        let reopened_metadata = reopened.metadata().context(RECEIPT_FILE_ERROR)?;
        ensure!(
            stable(&self.metadata, &reopened_metadata),
            RECEIPT_FILE_ERROR
        );
        validate_descriptor(&reopened, &reopened_metadata)?;
        ensure!(
            read_from_file(&reopened, self.metadata.len())? == self.bytes,
            RECEIPT_FILE_ERROR
        );
        let reopened_after_read = reopened.metadata().context(RECEIPT_FILE_ERROR)?;
        ensure!(
            stable(&self.metadata, &reopened_after_read),
            RECEIPT_FILE_ERROR
        );
        validate_descriptor(&reopened, &reopened_after_read)?;
        Ok(())
    }
}

pub(crate) fn open_bounded(path: &Path) -> Result<OpenReceiptFile> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
        .context(RECEIPT_FILE_ERROR)?;
    let metadata = file.metadata().context(RECEIPT_FILE_ERROR)?;
    validate_descriptor(&file, &metadata)?;
    ensure!(
        metadata.len() <= MAX_ACCEPTANCE_RECEIPT_BYTES,
        RECEIPT_FILE_ERROR
    );
    let bytes = read_from_file(&file, metadata.len())?;
    let final_metadata = file.metadata().context(RECEIPT_FILE_ERROR)?;
    ensure!(stable(&metadata, &final_metadata), RECEIPT_FILE_ERROR);
    validate_descriptor(&file, &final_metadata)?;
    Ok(OpenReceiptFile {
        path: path.to_owned(),
        file,
        metadata,
        bytes,
    })
}

fn read_from_file(file: &std::fs::File, length: u64) -> Result<Vec<u8>> {
    let mut bytes = vec![0_u8; usize::try_from(length)?];
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let count = file
            .read_at(&mut bytes[offset..], offset as u64)
            .context(RECEIPT_FILE_ERROR)?;
        ensure!(count > 0, RECEIPT_FILE_ERROR);
        offset += count;
    }
    Ok(bytes)
}

pub(crate) fn reject_duplicate_fields(bytes: &[u8]) -> Result<()> {
    let mut parser = serde_json::Deserializer::from_slice(bytes);
    parser
        .deserialize_any(DuplicateFieldVisitor)
        .context(RECEIPT_PARSE_ERROR)?;
    parser.end().context(RECEIPT_PARSE_ERROR)
}

#[derive(Clone, Copy)]
struct DuplicateFieldVisitor;

impl<'de> DeserializeSeed<'de> for DuplicateFieldVisitor {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for DuplicateFieldVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut fields = std::collections::BTreeSet::new();
        while let Some(field) = map.next_key::<String>()? {
            if !fields.insert(field) {
                return Err(de::Error::custom(RECEIPT_DUPLICATE_FIELD_ERROR));
            }
            map.next_value_seed(self)?;
        }
        Ok(())
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element_seed(self)?.is_some() {}
        Ok(())
    }

    fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_f64<E>(self, _: f64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_str<E>(self, _: &str) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_string<E>(self, _: String) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}
