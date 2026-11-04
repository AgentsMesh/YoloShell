use super::runtime_intake_test_support::RoutedFixture;

#[test]
fn routed_fixtures_use_unique_scopes_and_independent_writer_locks() {
    let first = RoutedFixture::new();
    let second = RoutedFixture::new();
    assert_ne!(
        first.identity.terminal_id,
        second.identity.terminal_id
    );
    assert_ne!(first.identity.runtime_epoch, second.identity.runtime_epoch);
    assert_ne!(first.paths.home(), second.paths.home());
    assert_ne!(first.router().test_key(), second.router().test_key());
    assert_ne!(first.runtime.test_key(), second.runtime.test_key());

    let first_entry = first.router().resolve(&first.identity).unwrap();
    let first_writer = first_entry
        .admission()
        .authority()
        .acquire_retirement_writer_fence()
        .unwrap();
    let second_entry = second.router().resolve(&second.identity).unwrap();
    let second_writer = second_entry
        .admission()
        .authority()
        .acquire_retirement_writer_fence()
        .unwrap();

    first_writer.verify_unsealed().unwrap();
    second_writer.verify_unsealed().unwrap();
}
