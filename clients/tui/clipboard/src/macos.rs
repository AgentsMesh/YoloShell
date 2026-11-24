use crate::files::{Item, file_urls};
use anyhow::{Context, Result, bail};
use std::ffi::{c_char, c_void};

unsafe extern "C" {
    fn yolo_read_pasteboard(
        name: *const c_char,
        copy: unsafe extern "C" fn(*mut Vec<u8>, *const c_void, usize),
        context: *mut Vec<u8>,
    ) -> u8;
}

unsafe extern "C" fn copy(context: *mut Vec<u8>, data: *const c_void, size: usize) {
    // The pasteboard calls this synchronously with the bytes of a live CFData.
    unsafe {
        (*context).extend_from_slice(std::slice::from_raw_parts(data.cast(), size));
    }
}

pub(super) async fn items() -> Result<Vec<Item>> {
    tokio::task::spawn_blocking(|| {
        let mut bytes = Vec::new();
        let tag = unsafe { yolo_read_pasteboard(std::ptr::null(), copy, &mut bytes) };
        match tag {
            b'F' => file_urls(std::str::from_utf8(&bytes)?),
            b'P' | b'J' => {
                let name = if tag == b'P' {
                    "clipboard.png"
                } else {
                    "clipboard.jpg"
                };
                Ok(vec![Item::Image { name, bytes }])
            }
            _ => bail!("Copy a file or image, then paste it here"),
        }
    })
    .await
    .context("Read macOS clipboard")?
}
