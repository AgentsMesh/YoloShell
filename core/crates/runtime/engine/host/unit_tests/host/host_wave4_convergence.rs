use crate::host::tests::*;
use super::*;

include!("../catalog/catalog_durable_read_wave4.rs");
include!("../catalog/catalog_durable_boundary_wave4.rs");
include!("../creation/receipt/creation_receipt_cleanup_wave4.rs");
include!("../creation/receipt/creation_receipt_publish_wave4.rs");
include!("../server/server_ingress_wave4.rs");
include!("../session/session_spawn_wave4.rs");
include!("../creation/ledger/creation_wave4.rs");
include!("host_wave5_convergence.rs");
