pub mod browser;
mod constants;
pub mod fixture;
pub mod peer;
pub mod source;

pub use browser::Browser;
pub use constants::*;
pub use fixture::Fixture;
pub fn deadline() -> tokio::time::Instant {
    tokio::time::Instant::now() + TIMEOUT
}
