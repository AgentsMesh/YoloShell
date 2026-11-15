use crate::claude_scenario::{AdapterResult, ClaudeHookAdapter};
use crate::claude_wire::ClaudeHookFrame;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::Path;

pub(crate) const JSON_LINE_ENDING: u8 = b'\n';

pub(crate) struct JsonLineHookAdapter<W> {
    writer: W,
}

impl<W> JsonLineHookAdapter<W> {
    pub(crate) fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl JsonLineHookAdapter<UnixStream> {
    pub(crate) fn connect(path: &Path) -> std::io::Result<Self> {
        UnixStream::connect(path).map(Self::new)
    }
}

impl<W: Write> ClaudeHookAdapter for JsonLineHookAdapter<W> {
    fn deliver(&mut self, frame: &ClaudeHookFrame) -> AdapterResult {
        serde_json::to_writer(&mut self.writer, frame)?;
        self.writer.write_all(&[JSON_LINE_ENDING])?;
        self.writer.flush()?;
        Ok(())
    }
}
