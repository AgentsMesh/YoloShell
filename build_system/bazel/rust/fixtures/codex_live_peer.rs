use std::io::{Read, Write};
use std::path::PathBuf;
use yolo_codex_live_peer_fixture_contract::{FRAME_HEADER_BYTES, SOCKET_ENVIRONMENT};

const MISSING_SOCKET: &str = "live peer socket environment is missing";
const OVERSIZED_PAYLOAD: &str = "live peer payload exceeds frame capacity";
const UNEXPECTED_RESPONSE_TAIL: &str = "live peer received bytes after acknowledgement";

fn main() -> std::io::Result<()> {
    let socket = environment_path(SOCKET_ENVIRONMENT, MISSING_SOCKET)?;
    let mut payload = Vec::new();
    std::io::stdin().read_to_end(&mut payload)?;
    let length =
        u32::try_from(payload.len()).map_err(|_| std::io::Error::other(OVERSIZED_PAYLOAD))?;
    let mut stream = std::os::unix::net::UnixStream::connect(socket)?;
    stream.write_all(&length.to_be_bytes())?;
    stream.write_all(&payload)?;
    let mut header = [0_u8; FRAME_HEADER_BYTES];
    stream.read_exact(&mut header)?;
    let mut response = vec![0_u8; u32::from_be_bytes(header) as usize];
    stream.read_exact(&mut response)?;
    let mut output = std::io::stdout().lock();
    output.write_all(&header)?;
    output.write_all(&response)?;
    output.flush()?;
    let mut trailing = [0_u8; 1];
    if stream.read(&mut trailing)? == usize::default() {
        Ok(())
    } else {
        Err(std::io::Error::other(UNEXPECTED_RESPONSE_TAIL))
    }
}

fn environment_path(name: &str, missing: &'static str) -> std::io::Result<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other(missing))
}
