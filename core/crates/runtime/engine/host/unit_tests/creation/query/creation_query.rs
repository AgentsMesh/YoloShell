use super::*;
use crate::model::{CreationQueryOutcome, CreationQueryResult};

#[path = "../../../test_support/unit/creation/creation_query_constants.rs"]
mod constants;
use self::constants::*;

#[path = "../../../test_support/unit/creation/creation_query.rs"]
mod support;
use support::*;
#[path = "creation_query_fences.rs"]
mod fences;
#[path = "../receipt/creation_query_receipt_validation.rs"]
mod receipt_validation;
#[path = "../receipt/creation_query_receipts.rs"]
mod receipts;
#[path = "creation_query_states.rs"]
mod states;
