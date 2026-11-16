struct FlushFails;

impl Write for FlushFails {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
}
