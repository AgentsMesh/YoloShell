use crate::constants::*;
use std::io;

pub(crate) struct ProfileName {
    prefix: String,
    suffix: String,
    module: bool,
    pub(crate) continuous: bool,
}

impl ProfileName {
    pub(crate) fn parse(pattern: &str, pid: u32, hostname: &str) -> io::Result<Self> {
        let continuous_markers = pattern.matches(CONTINUOUS_TOKEN).count();
        if continuous_markers > MAX_CONTINUOUS_MARKERS {
            return Err(invalid());
        }
        let expanded = pattern
            .replace(CONTINUOUS_TOKEN, "")
            .replace(HOST_TOKEN, hostname);
        let (before, after) = expanded.split_once(PID_TOKEN).ok_or_else(invalid)?;
        if pid == u32::default() || before.contains(FORMAT_MARKER) {
            return Err(invalid());
        }
        let (separator, suffix, module) = match after.split_once(MODULE_TOKEN) {
            Some((separator, suffix))
                if separator.starts_with(|value: char| !value.is_ascii_digit()) =>
            {
                (separator, suffix, true)
            }
            Some(_) => return Err(invalid()),
            None => (after, "", false),
        };
        if separator.contains(FORMAT_MARKER) || suffix.contains(FORMAT_MARKER) {
            return Err(invalid());
        }
        Ok(Self {
            prefix: format!("{before}{pid}{separator}"),
            suffix: suffix.to_owned(),
            module,
            continuous: continuous_markers == MAX_CONTINUOUS_MARKERS,
        })
    }

    pub(crate) fn matches(&self, candidate: &str) -> bool {
        if !self.module {
            return candidate == self.prefix;
        }
        let Some(module) = candidate
            .strip_prefix(&self.prefix)
            .and_then(|rest| rest.strip_suffix(&self.suffix))
        else {
            return false;
        };
        let Some((signature, pool)) = module.split_once(MODULE_SEPARATOR) else {
            return false;
        };
        [signature, pool]
            .into_iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    }
}

pub(crate) fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, INVALID_PATTERN)
}
