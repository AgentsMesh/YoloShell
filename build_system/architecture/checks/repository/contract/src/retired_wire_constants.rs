pub(super) const CORE_ROOT: &str = "core/crates/";
pub(super) const RUST_SOURCE: &str = "/src/";
pub(super) const RUST_EXTENSION: &str = ".rs";
pub(super) const RETIRED_IDENTIFIERS: &[&str] = &[
    "DETACH_SESSION",
    "ACK_OUTPUT",
    "RESUME_SESSION",
    "WRITER_LEASE",
    "TerminateRequest",
];
pub(super) const RETIRED_WIRE_VALUES: &[&str] = &[
    "detach_session",
    "ack_output",
    "resume_session",
    "writer_lease",
];
pub(super) const MESSAGE: &str =
    "runtime wire must use current exact requests and capabilities without retired method adapters";
