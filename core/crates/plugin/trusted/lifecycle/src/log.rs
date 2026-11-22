use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::Path;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::LifecycleContext;

const MAX_LOG_BYTES: usize = 1024 * 1024;
const READER_GRACE: Duration = Duration::from_millis(200);
const READER_POLL: Duration = Duration::from_millis(5);

pub(crate) fn private_dir(path: &Path) -> std::io::Result<()> {
    yolo_paths::directory::PrivateDirectory::ensure(path).map(|_| ()).map_err(std::io::Error::other)
}

pub(crate) fn create(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

pub(crate) fn spawn(pipe: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.take(MAX_LOG_BYTES as u64 + 1).read_to_end(&mut bytes);
        bytes
    })
}

pub(crate) fn write(
    mut file: &File,
    readers: Vec<JoinHandle<Vec<u8>>>,
    context: &LifecycleContext,
) {
    use std::io::Write;
    let deadline = Instant::now() + READER_GRACE;
    while Instant::now() < deadline && readers.iter().any(|reader| !reader.is_finished()) {
        std::thread::sleep(READER_POLL);
    }
    let mut bytes = Vec::new();
    for reader in readers {
        if reader.is_finished() {
            if let Ok(output) = reader.join() {
                bytes.extend(output);
            }
        }
    }
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    text = text.replace(
        context.hub_socket.to_string_lossy().as_ref(),
        "<YOLO_HUB_SOCKET>",
    );
    if let Some(hub_id) = context.hub_id {
        text = text.replace(&hub_id.to_string(), "<YOLO_HUB_ID>");
    }
    let mut end = text.len().min(MAX_LOG_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let _ = file.write_all(&text.as_bytes()[..end]);
}
