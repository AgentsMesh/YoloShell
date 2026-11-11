use super::super::procfs;
use super::linux_contract::{CandidatePending, QueryKind, WorkWindow};
use super::linux_io::io_pending;
use std::os::unix::fs::{FileTypeExt, MetadataExt};

pub(super) enum DescriptorValue {
    Holds,
    Excluded,
    Pending(CandidatePending),
}

pub(super) struct DescriptorQuery {
    pub(super) value: DescriptorValue,
    pub(super) examined: usize,
}

pub(super) fn holds_device(pid: u32, device: u64, window: WorkWindow) -> DescriptorQuery {
    if window.limit == usize::default() {
        return pending(CandidatePending::Budget(window.exhausted), usize::default());
    }
    let descriptors = match std::fs::read_dir(procfs::file_descriptors_path(pid)) {
        Ok(descriptors) => descriptors,
        Err(error) => return pending(io_pending(QueryKind::Descriptors, &error), usize::default()),
    };
    let mut examined = usize::default();
    let mut uncertainty = None;
    for descriptor in descriptors {
        if examined >= window.limit {
            return pending(CandidatePending::Budget(window.exhausted), examined);
        }
        examined += 1;
        let descriptor = match descriptor {
            Ok(descriptor) => descriptor,
            Err(error) => {
                uncertainty = Some(io_pending(QueryKind::Descriptors, &error));
                continue;
            }
        };
        if descriptor
            .file_name()
            .to_string_lossy()
            .parse::<u32>()
            .is_err()
        {
            uncertainty = Some(CandidatePending::Query(QueryKind::Descriptors));
            continue;
        }
        match std::fs::metadata(descriptor.path()) {
            Ok(metadata) if metadata.file_type().is_char_device() && metadata.rdev() == device => {
                return DescriptorQuery {
                    value: DescriptorValue::Holds,
                    examined,
                };
            }
            Ok(_) => {}
            Err(error) => uncertainty = Some(io_pending(QueryKind::Descriptors, &error)),
        }
    }
    DescriptorQuery {
        value: uncertainty
            .map(DescriptorValue::Pending)
            .unwrap_or(DescriptorValue::Excluded),
        examined,
    }
}

fn pending(reason: CandidatePending, examined: usize) -> DescriptorQuery {
    DescriptorQuery {
        value: DescriptorValue::Pending(reason),
        examined,
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/linux/pty_session_linux_fds_tests.rs"]
mod tests;
