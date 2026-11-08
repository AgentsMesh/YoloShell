use anyhow::{Context, Result, ensure};

use crate::constants::{
    CURRENT_COMPONENT, MAX_ENTRY_NAME_BYTES, PARENT_COMPONENT, PATH_SEPARATOR, ROOT_ENTRY,
    WINDOWS_PATH_SEPARATOR,
};

pub(crate) struct ValidatedPath {
    pub path: String,
    pub alias: String,
    pub has_directory_suffix: bool,
}

pub(crate) fn validate(raw: &[u8]) -> Result<ValidatedPath> {
    ensure!(!raw.is_empty(), "ZIP entry name is empty");
    ensure!(
        raw.len() <= MAX_ENTRY_NAME_BYTES,
        "ZIP entry name exceeds budget"
    );
    ensure!(raw.is_ascii(), "ZIP entry name must be raw ASCII");
    ensure!(
        !raw.contains(&WINDOWS_PATH_SEPARATOR),
        "ZIP entry name contains a backslash"
    );
    ensure!(
        raw.first() != Some(&PATH_SEPARATOR),
        "ZIP entry name is absolute"
    );
    let path = String::from_utf8(raw.to_vec()).context("decode ASCII ZIP entry name")?;
    let has_directory_suffix = raw.last() == Some(&PATH_SEPARATOR);
    let component_path = if has_directory_suffix {
        path.strip_suffix(char::from(PATH_SEPARATOR))
            .context("strip ZIP directory suffix")?
    } else {
        path.as_str()
    };
    ensure!(
        !component_path.is_empty(),
        "ZIP entry path has no components"
    );
    let mut components = component_path.split(char::from(PATH_SEPARATOR));
    ensure!(
        components.next() == Some(ROOT_ENTRY),
        "ZIP entry is outside the Yolo.app root"
    );
    for component in components {
        validate_component(component)?;
    }
    let mut alias = component_path.to_owned();
    alias.make_ascii_lowercase();
    Ok(ValidatedPath {
        path,
        alias,
        has_directory_suffix,
    })
}

fn validate_component(component: &str) -> Result<()> {
    ensure!(
        !component.is_empty(),
        "ZIP entry has an empty path component"
    );
    ensure!(
        component != CURRENT_COMPONENT && component != PARENT_COMPONENT,
        "ZIP entry has a traversal component"
    );
    ensure!(
        !matches!(component.as_bytes().last(), Some(b'.' | b' ')),
        "ZIP entry component has an unsafe suffix"
    );
    ensure!(
        component.bytes().all(allowed_component_byte),
        "ZIP entry component contains an unsafe ASCII byte"
    );
    Ok(())
}

fn allowed_component_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
}
