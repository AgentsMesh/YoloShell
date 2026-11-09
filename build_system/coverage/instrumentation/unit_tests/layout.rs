#[path = "../test_support/constants.rs"]
mod constants;
#[path = "../test_support/macho.rs"]
mod macho;

mod layout_boundaries;
#[path = "../test_support/layout_constants.rs"]
mod layout_constants;
#[path = "../test_support/layout_fixture.rs"]
mod layout_fixture;
mod layout_malformed;
mod layout_overflow;
mod layout_overlap;
mod layout_rejections;
mod layout_valid;
