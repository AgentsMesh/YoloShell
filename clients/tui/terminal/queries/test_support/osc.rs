#[path = "osc_constants.rs"]
mod constants;
pub use constants::*;

pub fn title(raw_bytes: usize, termination: &[u8]) -> Vec<u8> {
    sequence(TITLE_PREFIX, raw_bytes - TITLE_PARAMETER_BYTES, termination)
}

pub fn clipboard(raw_bytes: usize, termination: &[u8]) -> Vec<u8> {
    sequence(
        CLIPBOARD_PREFIX,
        raw_bytes - CLIPBOARD_PARAMETER_BYTES,
        termination,
    )
}

fn sequence(prefix: &[u8], payload_bytes: usize, termination: &[u8]) -> Vec<u8> {
    let mut bytes = prefix.to_vec();
    bytes.resize(bytes.len() + payload_bytes, PAYLOAD_BYTE);
    bytes.extend_from_slice(termination);
    bytes
}

#[derive(Default)]
pub struct OscCollector {
    pub records: Vec<Vec<Vec<u8>>>,
}

impl vte::Perform for OscCollector {
    fn osc_dispatch(&mut self, params: &[&[u8]], _: bool) {
        self.records
            .push(params.iter().map(|value| value.to_vec()).collect());
    }
}

#[derive(Default)]
pub struct ScreenCallbacks {
    pub titles: Vec<Vec<u8>>,
    pub clipboard: Vec<Vec<u8>>,
}

impl vt100::Callbacks for ScreenCallbacks {
    fn set_window_title(&mut self, _: &mut vt100::Screen, title: &[u8]) {
        self.titles.push(title.to_vec());
    }
    fn copy_to_clipboard(&mut self, _: &mut vt100::Screen, selector: &[u8], data: &[u8]) {
        assert_eq!(selector, CLIPBOARD_SELECTOR);
        self.clipboard.push(data.to_vec());
    }
}
