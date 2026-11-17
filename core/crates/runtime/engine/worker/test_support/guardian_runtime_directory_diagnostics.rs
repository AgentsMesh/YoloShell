use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const MAX_DIAGNOSTIC_DESCRIPTORS: libc::c_int = 4096;
const MAX_DIAGNOSTIC_ENTRIES: usize = 128;

pub(super) fn capture(directory: &Path) -> String {
    let entries = std::fs::read_dir(directory).map(|entries| {
        let mut entries = entries
            .take(MAX_DIAGNOSTIC_ENTRIES)
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| format!("{entry:?}"));
        entries
    });
    let listeners = (0..MAX_DIAGNOSTIC_DESCRIPTORS)
        .filter_map(|descriptor| listener_in_directory(descriptor, directory))
        .collect::<Vec<_>>();
    format!(
        "namespace={}, entries={entries:?}, current_process_listeners={listeners:?}",
        directory.display()
    )
}

fn listener_in_directory(descriptor: libc::c_int, directory: &Path) -> Option<(i32, String)> {
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of_val(&address) as libc::socklen_t;
    if unsafe {
        libc::getsockname(
            descriptor,
            std::ptr::from_mut(&mut address).cast(),
            &mut length,
        )
    } != 0
        || address.sun_family != libc::AF_UNIX as libc::sa_family_t
    {
        return None;
    }
    let bytes = address
        .sun_path
        .iter()
        .map(|byte| *byte as u8)
        .take_while(|byte| *byte != 0)
        .collect::<Vec<_>>();
    let path = Path::new(std::ffi::OsStr::from_bytes(&bytes));
    if path.parent()?.canonicalize().ok()? != directory.canonicalize().ok()? {
        return None;
    }
    let mut listening: libc::c_int = 0;
    let mut length = std::mem::size_of_val(&listening) as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            descriptor,
            libc::SOL_SOCKET,
            libc::SO_ACCEPTCONN,
            std::ptr::from_mut(&mut listening).cast(),
            &mut length,
        )
    };
    (result == 0 && listening != 0).then(|| (descriptor, path.display().to_string()))
}
