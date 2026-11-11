mod constants;
mod key;
mod keypad;
mod mouse;
mod mouse_constants;
mod paste;

pub use key::{encode_key, encode_key_default};
pub use mouse::encode_mouse;
pub use paste::encode_paste;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputError {
    UnsupportedKey,
    PayloadTooLarge,
    UnsafePaste,
    UnsupportedMouse,
    MouseCoordinate,
}

impl std::fmt::Display for InputError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedKey => constants::UNSUPPORTED_KEY,
            Self::PayloadTooLarge => constants::PAYLOAD_TOO_LARGE,
            Self::UnsafePaste => constants::UNSAFE_PASTE,
            Self::UnsupportedMouse => mouse_constants::UNSUPPORTED_MOUSE,
            Self::MouseCoordinate => mouse_constants::MOUSE_COORDINATE,
        })
    }
}
impl std::error::Error for InputError {}
