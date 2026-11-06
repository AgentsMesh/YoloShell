pub(super) use crate::process::PreparedRetirementCustodyFixture as ClosedCustody;

pub(super) fn create() -> ClosedCustody {
    crate::process::prepared_retirement_custody_fixture()
}
