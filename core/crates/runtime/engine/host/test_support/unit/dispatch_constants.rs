use crate::host::tests::*;
const MALFORMED_DISPATCH_ID_PREFIX: &str = "malformed-dispatch-";
const MALFORMED_DISPATCH_PAYLOAD: &str = "payload-is-not-a-request-map";
const RETIRED_HOST_METHODS: [&str; 3] = ["detach_session", "ack_output", "resume_session"];
