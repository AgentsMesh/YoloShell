const FIXTURE_SESSION_NAME: &str = "failed resume rearm process fixture";
const FIXTURE_ENCODE_CONTEXT: &str = "encode failed-resume rearm fixture evidence";
const FIXTURE_DECODE_CONTEXT: &str = "decode failed-resume rearm fixture evidence";
const FIXTURE_HOME_MISMATCH: &str = "failed-resume rearm fixture home changed";
const FIXTURE_BOUNDARY_MISMATCH: &str = "failed-resume rearm crash boundary was not armed";
const FIXTURE_CRASH_NOT_DELIVERED: &str = "failed-resume rearm crash was not delivered";
const FIXTURE_PREPARED_STATE_MISMATCH: &str = "failed-resume rearm prepared state changed";
const FIXTURE_CRASH_STATE_MISMATCH: &str = "failed-resume rearm durable crash state is incorrect";
const FIXTURE_GENERATION_MISMATCH: &str =
    "failed-resume rearm controller generation did not advance";
const FIXTURE_RECONCILE_MISMATCH: &str = "failed-resume rearm reconciliation count is incorrect";
const FIXTURE_RECOVERY_MISMATCH: &str = "failed-resume rearm recovery state is incorrect";
const FIXTURE_RETRY_MISMATCH: &str = "failed-resume rearm retry changed durable state";
const FIXTURE_SOCKET_RESIDUE: &str = "failed-resume rearm worker socket remains";
const EXPECTED_RECONCILE_UNREACHABLE: usize = 1;
