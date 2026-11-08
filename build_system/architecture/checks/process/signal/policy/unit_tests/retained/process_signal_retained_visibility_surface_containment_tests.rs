use super::process_signal_retained_visibility_containment_tests::{
    assert_child_count, assert_external_count,
};

const CLEAN: usize = 0;
const REJECTED: usize = 1;
const LOCAL_PREFIX: &str = "struct RetainedExactProcess;\n\
    pub(super) struct Aggregate { target: RetainedExactProcess }\n";
const EXTERNAL_IMPORT: &str = "use crate::owner::authority::Aggregate;\n";

#[test]
fn restricted_functions_and_values_can_expose_contained_aggregates() {
    for surface in [
        "pub(super) fn expose(_: Aggregate) {}",
        "pub(in crate::owner) const TARGET: Option<Aggregate> = None;",
        "pub(super) static TARGET: Option<Aggregate> = None;",
    ] {
        assert_local(surface, CLEAN);
    }
    assert_local("pub(super) fn expose(_: RetainedExactProcess) {}", REJECTED);
    assert_local("pub(crate) fn expose(_: Aggregate) {}", REJECTED);
}

#[test]
fn fields_require_contained_noncanonical_types() {
    assert_local(
        "pub(super) struct Holder { pub(super) target: Aggregate }",
        CLEAN,
    );
    assert_local(
        "pub(super) struct Holder { pub(super) target: RetainedExactProcess }",
        REJECTED,
    );
    assert_local(
        "pub(super) struct Holder { pub(crate) target: Aggregate }",
        CLEAN,
    );
}

#[test]
fn restricted_traits_and_associated_types_preserve_containment() {
    assert_local(
        "pub(super) trait Surface { fn target() -> Aggregate; }",
        CLEAN,
    );
    assert_local(
        "pub(super) trait Surface { fn target() -> RetainedExactProcess; }",
        REJECTED,
    );
    assert_local(
        "pub(super) trait Surface { type Target; }\n\
         struct Holder;\n\
         impl Surface for Holder { type Target = Aggregate; }",
        CLEAN,
    );
    assert_local(
        "struct Holder;\n\
         impl Holder { pub(super) type Target = RetainedExactProcess; }",
        REJECTED,
    );
}

#[test]
fn reexports_distinguish_aggregate_and_canonical_aliases() {
    let prefix = "struct RetainedExactProcess;\n\
        mod inner {\n\
            pub(in crate::owner) struct Aggregate {\n\
                target: super::RetainedExactProcess\n\
            }\n\
        }\n";
    for surface in [
        "pub(super) use inner::Aggregate;",
        "pub(super) use inner::Aggregate as Shared;",
    ] {
        assert_child_count(&format!("{prefix}{surface}"), CLEAN);
    }
    assert_child_count(
        &format!("{prefix}pub(super) use self::RetainedExactProcess as Shared;"),
        REJECTED,
    );
}

#[test]
fn malformed_or_broad_visibility_fails_closed() {
    for surface in [
        "pub(in super::child) fn expose(_: Aggregate) {}",
        "pub(in crate) fn expose(_: Aggregate) {}",
        "pub fn expose(_: Aggregate) {}",
    ] {
        assert_local(surface, REJECTED);
    }
}

#[test]
fn cross_file_aggregate_surfaces_respect_canonical_resolution() {
    for surface in [
        "pub(super) fn expose(_: Aggregate) {}",
        "pub(super) struct Holder { pub(super) target: Aggregate }",
    ] {
        let source = format!("{EXTERNAL_IMPORT}{surface}");
        assert_external_count(&source, false, CLEAN);
        assert_external_count(&source, true, REJECTED);
    }
    let alias = "pub(super) use crate::owner::authority::Aggregate as Shared;";
    assert_external_count(alias, false, CLEAN);
    assert_external_count(alias, true, REJECTED);
}

fn assert_local(surface: &str, expected: usize) {
    assert_child_count(&format!("{LOCAL_PREFIX}{surface}"), expected);
}
