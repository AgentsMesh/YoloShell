mod admission;
mod catalogue;
mod constants;
mod dispatcher;
mod guard;
mod pages;
mod source;

pub use dispatcher::ScopedDispatcher;
pub use guard::while_authorized;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
