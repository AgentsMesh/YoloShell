//! Demonstrates an authored crate root alongside Bazel-generated schema modules.
pub mod authored_capnp {
    include!("generated/authored_capnp.rs");
}

pub fn enabled(message: &capnp::message::Builder<capnp::message::HeapAllocator>) -> bool {
    message
        .get_root_as_reader::<authored_capnp::envelope::Reader<'_>>()
        .unwrap()
        .get_enabled()
}
