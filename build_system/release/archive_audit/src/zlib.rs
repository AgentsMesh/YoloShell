use anyhow::{Context, Result, ensure};
use crc32fast::Hasher;
use sha2::{Digest, Sha256};
use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};

use crate::constants::PAYLOAD_BUFFER_BYTES;
use yolo_release_foundation::digest::hex_digest;

const RAW_DEFLATE_WINDOW_BITS: c_int = -15;
const ZLIB_NO_FLUSH: c_int = 0;
const ZLIB_OK: c_int = 0;
const ZLIB_STREAM_END: c_int = 1;

type Allocate = unsafe extern "C" fn(*mut c_void, c_uint, c_uint) -> *mut c_void;
type Free = unsafe extern "C" fn(*mut c_void, *mut c_void);

#[repr(C)]
struct ZStream {
    next_in: *mut u8,
    avail_in: c_uint,
    total_in: c_ulong,
    next_out: *mut u8,
    avail_out: c_uint,
    total_out: c_ulong,
    message: *mut c_char,
    state: *mut c_void,
    allocate: Option<Allocate>,
    free: Option<Free>,
    opaque: *mut c_void,
    data_type: c_int,
    adler: c_ulong,
    reserved: c_ulong,
}

#[link(name = "z")]
unsafe extern "C" {
    fn zlibVersion() -> *const c_char;
    fn inflateInit2_(
        stream: *mut ZStream,
        window_bits: c_int,
        version: *const c_char,
        stream_size: c_int,
    ) -> c_int;
    fn inflate(stream: *mut ZStream, flush: c_int) -> c_int;
    fn inflateEnd(stream: *mut ZStream) -> c_int;
}

pub(crate) fn validate(
    compressed: &mut [u8],
    expected_size: u64,
    expected_crc32: u32,
) -> Result<String> {
    let mut stream: ZStream = unsafe { std::mem::zeroed() };
    stream.next_in = compressed.as_mut_ptr();
    stream.avail_in = c_uint::try_from(compressed.len()).context("deflate input length")?;
    let stream_size = c_int::try_from(std::mem::size_of::<ZStream>())?;
    let initialize = unsafe {
        inflateInit2_(
            &mut stream,
            RAW_DEFLATE_WINDOW_BITS,
            zlibVersion(),
            stream_size,
        )
    };
    ensure!(
        initialize == ZLIB_OK,
        "initialize bounded raw deflate decoder"
    );
    let result = inflate_bounded(&mut stream, expected_size, expected_crc32);
    let finalize = unsafe { inflateEnd(&mut stream) };
    if result.is_ok() {
        ensure!(finalize == ZLIB_OK, "finalize bounded raw deflate decoder");
    }
    result
}

fn inflate_bounded(
    stream: &mut ZStream,
    expected_size: u64,
    expected_crc32: u32,
) -> Result<String> {
    let mut output = [0_u8; PAYLOAD_BUFFER_BYTES];
    let output_capacity = c_uint::try_from(output.len())?;
    let mut observed_size = 0_u64;
    let mut crc = Hasher::new();
    let mut digest = Sha256::new();
    loop {
        stream.next_out = output.as_mut_ptr();
        stream.avail_out = output_capacity;
        let status = unsafe { inflate(stream, ZLIB_NO_FLUSH) };
        let produced = usize::try_from(output_capacity - stream.avail_out)?;
        crc.update(&output[..produced]);
        digest.update(&output[..produced]);
        observed_size = observed_size
            .checked_add(produced as u64)
            .context("deflate output length overflow")?;
        ensure!(
            observed_size <= expected_size,
            "deflate output exceeds declared bounded size"
        );
        if status == ZLIB_STREAM_END {
            break;
        }
        ensure!(
            status == ZLIB_OK && produced > 0,
            "invalid or non-progressing raw deflate stream"
        );
    }
    ensure!(
        stream.avail_in == 0,
        "deflate stream has trailing input bytes"
    );
    ensure!(
        observed_size == expected_size,
        "deflate output differs from declared size"
    );
    ensure!(crc.finalize() == expected_crc32, "ZIP payload CRC mismatch");
    Ok(hex_digest(digest.finalize()))
}
