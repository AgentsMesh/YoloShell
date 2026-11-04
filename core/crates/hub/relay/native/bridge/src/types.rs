#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Identity { pub high: u64, pub low: u64 }
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Text { pub data: *const u8, pub length: usize }
impl Default for Text { fn default() -> Self { Self { data: std::ptr::null(), length: 0 } } }
#[repr(C)]
#[derive(Default)]
pub struct Buffer { pub data: *mut u8, pub length: usize }
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Request {
    pub launch: Identity, pub hub: Identity, pub terminal: Identity, pub runtime: Identity,
    pub request: Identity, pub agent: Identity, pub execution: Identity, pub stream: Identity,
    pub sequence: u64, pub action: u32, pub prompt: Text,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Outcome { pub kind: u32, pub state: u32, pub failure: u32, pub reason: u32 }
#[repr(C)]
#[derive(Default)]
pub struct Decoded { pub kind: u32, pub request: Request, pub outcome: Outcome, pub status: u32, pub prompt: Buffer, pub holder: Buffer }
