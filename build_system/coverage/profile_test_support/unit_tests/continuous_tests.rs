use super::{ProfileName, discard_pattern};

const HOST: &str = "continuous-profile-host";
const PID: u32 = 4123;
const NAME: &str = "continuous-profile-host-4123-17_0.profraw";
const CONTENT: &[u8] = b"must remain available for strict collector validation";
const PATTERNS: [&str; 3] = [
    "%h-%p-%m.profraw%c",
    "%c%h-%p-%m.profraw",
    "%h-%p-%m%c.profraw",
];

#[test]
fn continuous_markers_are_position_independent_and_preserve_every_profile() {
    for pattern in PATTERNS {
        let matcher = ProfileName::parse(pattern, PID, HOST).unwrap();
        assert!(matcher.continuous);
        assert!(matcher.matches(NAME));
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join(NAME);
        std::fs::write(&file, CONTENT).unwrap();
        assert_eq!(
            discard_pattern(&root.path().join(pattern), PID, HOST).unwrap(),
            0
        );
        assert_eq!(std::fs::read(&file).unwrap(), CONTENT);
        std::fs::write(&file, []).unwrap();
        assert_eq!(
            discard_pattern(&root.path().join(pattern), PID, HOST).unwrap(),
            0
        );
        assert_eq!(std::fs::metadata(&file).unwrap().len(), 0);
    }
}

#[test]
fn continuous_mode_does_not_hide_invalid_or_ambiguous_identity_templates() {
    for pattern in [
        "%h-%p-%m%c%c.profraw",
        "%h-%m.profraw%c",
        "%h-%p%x.profraw%c",
        "%h-%p%m.profraw%c",
    ] {
        assert!(ProfileName::parse(pattern, PID, HOST).is_err(), "{pattern}");
    }
    assert!(
        !ProfileName::parse("%h-%p-%m.profraw", PID, HOST)
            .unwrap()
            .continuous
    );
}
