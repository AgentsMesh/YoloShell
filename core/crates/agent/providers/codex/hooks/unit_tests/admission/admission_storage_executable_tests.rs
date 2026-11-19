#[path = "../../test_support/admission.rs"]
mod support;
use support::*;

const HOOK_REPLACEMENT: &str = "hook-replacement";
const REAL_REPLACEMENT: &str = "real-replacement";

#[test]
fn every_executable_content_or_file_identity_change_conflicts() {
    for role in ExecutableRole::ALL {
        for contents in [EXECUTABLE_CONTENT, CHANGED_EXECUTABLE_CONTENT] {
            let fixture = Fixture::new();
            let storage = fixture.storage();
            storage.publish(&fixture.record()).unwrap();
            let replacement = fixture.alternate_executable(role.replacement_name(), contents);
            std::fs::rename(replacement, role.path(&fixture)).unwrap();
            assert!(storage.publish(&fixture.record()).is_err(), "{role:?}");
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ExecutableRole {
    Hook,
    Real,
}

impl ExecutableRole {
    const ALL: &'static [Self] = &[Self::Hook, Self::Real];

    fn path(self, fixture: &Fixture) -> &std::path::Path {
        match self {
            Self::Hook => &fixture.request.hook_executable,
            Self::Real => &fixture.request.real_executable,
        }
    }

    const fn replacement_name(self) -> &'static str {
        match self {
            Self::Hook => HOOK_REPLACEMENT,
            Self::Real => REAL_REPLACEMENT,
        }
    }
}
