use crate::constants::*;
use std::{net::IpAddr, path::Path};

pub(crate) fn host(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_HOST_BYTES {
        return false;
    }
    if value.parse::<IpAddr>().is_ok() {
        return true;
    }
    let dns = value.strip_suffix('.').unwrap_or(value);
    dns.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= MAX_DNS_LABEL_BYTES
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}
pub(crate) fn username(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_USERNAME_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}
pub(crate) fn socket(value: &Path) -> bool {
    let Some(text) = value.to_str() else {
        return false;
    };
    value.is_absolute()
        && text.len() <= MAX_SOCKET_BYTES
        && !text.chars().any(char::is_control)
        && text.strip_prefix('/').is_some_and(|relative| {
            relative
                .split('/')
                .all(|component| !component.is_empty() && component != "." && component != "..")
        })
}
