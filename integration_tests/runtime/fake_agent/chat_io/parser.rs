use super::constants::*;

#[derive(Default)]
pub struct Editor {
    pending: Vec<u8>,
    draft: Vec<u8>,
    paste: bool,
}
impl Editor {
    pub fn feed(&mut self, input: &[u8]) -> Result<Vec<Vec<u8>>, &'static str> {
        self.pending.extend_from_slice(input);
        let mut submitted = Vec::new();
        while !self.pending.is_empty() {
            let delimiter = if self.paste { PASTE_END } else { PASTE_START };
            if self.pending.starts_with(delimiter) {
                self.pending.drain(..delimiter.len());
                self.paste = !self.paste;
                continue;
            }
            if delimiter.starts_with(&self.pending) {
                break;
            }
            let byte = self.pending.remove(0);
            if !self.paste && byte == b'\r' {
                submitted.push(std::mem::take(&mut self.draft));
            } else {
                if self.draft.len() >= MAX_INPUT_BYTES {
                    return Err(INPUT_LIMIT);
                }
                self.draft.push(byte);
            }
        }
        Ok(submitted)
    }
    pub fn draft(&self) -> &[u8] {
        &self.draft
    }
}
