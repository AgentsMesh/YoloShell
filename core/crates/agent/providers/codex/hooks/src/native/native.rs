#[path = "native_bridge.rs"]
mod bridge;
#[path = "native_environment.rs"]
mod environment;
#[path = "native_framing.rs"]
mod framing;
#[path = "native_spool.rs"]
mod spool;
#[path = "native_wire.rs"]
mod wire;

use super::{CodexExecutableAttestation, CodexHookInput, MAX_INPUT_BYTES};
use super::{
    constants, framing as wire_framing, intake_constants, intake_storage, mapping, read_stdin,
};
use environment as native_environment;

pub use super::constants::{
    NATIVE_CALLBACK_SHELLS, NATIVE_DATA_DIRECTORY_NAME, NATIVE_DATA_HOME_ENV,
    NATIVE_HOOK_EXECUTABLE_ENV, NATIVE_HOOK_HOME_ENV, NATIVE_INVOCATION_DOMAIN,
    NATIVE_SCHEMA_VERSION,
};
pub use bridge::run_native;
pub use framing::{
    CODEX_NATIVE_MAX_REQUEST_BYTES, read_native_ack, read_native_request, write_native_ack,
    write_native_request,
};
pub use spool::{CodexNativeOccurrence, CodexNativeSpool};
pub use wire::{CodexNativeAck, CodexNativeAckCode, CodexNativeRequest};

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/native/native_admission_tests.rs"]
mod admission_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/native/native_bridge_tests.rs"]
mod bridge_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/native/native_environment_tests.rs"]
mod environment_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/native/native_framing_tests.rs"]
mod framing_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/native/native_spool_tests.rs"]
mod spool_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../test_support/native.rs"]
mod test_support;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/native/native_wire_tests.rs"]
mod wire_tests;
