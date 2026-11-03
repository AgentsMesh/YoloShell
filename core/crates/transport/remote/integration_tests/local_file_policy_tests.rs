use super::owner_and_mode_are_safe;

const EXPECTED_OWNER: u32 = 501;
const FOREIGN_OWNER: u32 = EXPECTED_OWNER + 1;
const ROOT_OWNER: u32 = 0;
const SAFE_MODE: u32 = 0o600;
const GROUP_WRITE: u32 = 0o020;

#[test]
fn owner_policy_accepts_only_the_expected_owner_or_explicit_root() {
    assert!(owner_and_mode_are_safe(
        EXPECTED_OWNER,
        EXPECTED_OWNER,
        SAFE_MODE,
        GROUP_WRITE,
        false,
    ));
    assert!(!owner_and_mode_are_safe(
        FOREIGN_OWNER,
        EXPECTED_OWNER,
        SAFE_MODE,
        GROUP_WRITE,
        false,
    ));
    assert!(!owner_and_mode_are_safe(
        FOREIGN_OWNER,
        EXPECTED_OWNER,
        SAFE_MODE,
        GROUP_WRITE,
        true,
    ));
    assert!(!owner_and_mode_are_safe(
        ROOT_OWNER,
        EXPECTED_OWNER,
        SAFE_MODE,
        GROUP_WRITE,
        false,
    ));
    assert!(owner_and_mode_are_safe(
        ROOT_OWNER,
        EXPECTED_OWNER,
        SAFE_MODE,
        GROUP_WRITE,
        true,
    ));
}

#[test]
fn denied_mode_overrides_every_accepted_owner() {
    for owner in [EXPECTED_OWNER, ROOT_OWNER] {
        assert!(!owner_and_mode_are_safe(
            owner,
            EXPECTED_OWNER,
            SAFE_MODE | GROUP_WRITE,
            GROUP_WRITE,
            true,
        ));
    }
}
