use super::{FALSE, TRUE};
use std::path::Path;

#[test]
fn platform_exit_fixtures_exist() {
    assert!(Path::new(TRUE).is_file());
    assert!(Path::new(FALSE).is_file());
}
