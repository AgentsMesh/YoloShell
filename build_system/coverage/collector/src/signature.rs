use crate::coverage_json::*;
use yolo_coverage_protocol::{digest_bytes, encode_fields};

const SIGNATURE_VERSION: u64 = 1;

pub fn identity(name: &str, filenames: &[String]) -> Result<String, String> {
    if name.is_empty() {
        return Err("coverage function has an empty name".to_owned());
    }
    let mut fields = Vec::with_capacity(filenames.len().saturating_add(1));
    fields.push(name);
    fields.extend(filenames.iter().map(String::as_str));
    encode_fields(&fields)
}

pub fn mapping(function: &Function, branch_found: u64) -> Result<String, String> {
    let mut bytes = Vec::new();
    append(&mut bytes, SIGNATURE_VERSION);
    append_length(&mut bytes, function.regions.len(), "region")?;
    for region in &function.regions {
        for index in [
            LINE_START_INDEX,
            COLUMN_START_INDEX,
            LINE_END_INDEX,
            COLUMN_END_INDEX,
            FILE_ID_INDEX,
            EXPANDED_FILE_ID_INDEX,
            KIND_INDEX,
        ] {
            append(&mut bytes, region[index]);
        }
    }
    append_length(&mut bytes, function.branches.len(), "branch")?;
    for branch in &function.branches {
        for index in [
            LINE_START_INDEX,
            COLUMN_START_INDEX,
            LINE_END_INDEX,
            COLUMN_END_INDEX,
            BRANCH_FILE_ID_INDEX,
            BRANCH_EXPANDED_FILE_ID_INDEX,
            BRANCH_KIND_INDEX,
        ] {
            append(&mut bytes, branch[index]);
        }
    }
    append(&mut bytes, branch_found);
    Ok(digest_bytes(&bytes))
}

fn append(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn append_length(bytes: &mut Vec<u8>, length: usize, label: &str) -> Result<(), String> {
    let value = u64::try_from(length).map_err(|_| format!("{label} map is too large"))?;
    append(bytes, value);
    Ok(())
}
