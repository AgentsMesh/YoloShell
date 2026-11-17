use super::*;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStringExt;

const TEST_PROGRAM: &str = "/bin/sh";
const EXPLICIT_LOCALES: [&str; 5] = ["C", "POSIX", "C.UTF-8", "en_US.UTF-8", "invalid-locale"];
const LC_TIME_ENV: &str = "LC_TIME";
const LC_NUMERIC_ENV: &str = "LC_NUMERIC";
const CUSTOM_ENV: &str = "FUTURE_AGENT_HOME";
const CUSTOM_VALUE: &str = "/agent/data";
const EXPLICIT_VALUE: &str = "explicit";
const INHERITED_VALUE: &str = "inherited";
const NON_UNICODE_VALUE: &[u8] = &[0xff];

fn command() -> CommandBuilder {
    let mut command = CommandBuilder::new(TEST_PROGRAM);
    command.env_clear();
    command
}

#[test]
fn sparse_service_environment_gets_capabilities_and_only_character_locale() {
    let mut command = command();
    command.env(LC_TIME_ENV, EXPLICIT_VALUE);
    command.env(LC_NUMERIC_ENV, EXPLICIT_VALUE);
    configure(&mut command, &BTreeMap::new());
    for (key, value) in CAPABILITY_DEFAULTS {
        assert_eq!(command.get_env(key), Some(OsStr::new(value)));
    }
    assert_eq!(
        command.get_env(LC_CTYPE_ENV),
        Some(OsStr::new(DEFAULT_CHARACTER_LOCALE))
    );
    for key in [LANG_ENV, LC_ALL_ENV] {
        assert_eq!(command.get_env(key), None);
    }
    for key in [LC_TIME_ENV, LC_NUMERIC_ENV] {
        assert_eq!(command.get_env(key), Some(OsStr::new(EXPLICIT_VALUE)));
    }
    let configured = command.clone();
    configure(&mut command, &BTreeMap::new());
    assert_eq!(command, configured);
}

#[test]
fn empty_values_are_unset_for_defaults() {
    let mut command = command();
    for key in CHARACTER_LOCALE_KEYS
        .into_iter()
        .chain(CAPABILITY_DEFAULTS.map(|(key, _)| key))
    {
        command.env(key, "");
    }
    configure(&mut command, &BTreeMap::new());
    assert_eq!(
        command.get_env(LC_CTYPE_ENV),
        Some(OsStr::new(DEFAULT_CHARACTER_LOCALE))
    );
    for (key, value) in CAPABILITY_DEFAULTS {
        assert_eq!(command.get_env(key), Some(OsStr::new(value)));
    }
    for key in [LANG_ENV, LC_ALL_ENV] {
        assert_eq!(command.get_env(key), Some(OsStr::new("")));
    }
}

#[test]
fn explicit_locale_at_any_precedence_level_is_preserved_byte_for_byte() {
    for key in CHARACTER_LOCALE_KEYS {
        for value in EXPLICIT_LOCALES {
            let mut command = command();
            for other in CHARACTER_LOCALE_KEYS {
                command.env(other, "");
            }
            command.env(key, value);
            configure(&mut command, &BTreeMap::new());
            assert_eq!(command.get_env(key), Some(OsStr::new(value)));
            if key != LC_CTYPE_ENV {
                assert_eq!(command.get_env(LC_CTYPE_ENV), Some(OsStr::new("")));
            }
        }
    }
}

#[test]
fn defaults_use_the_merged_environment_and_do_not_override_launch_values() {
    for inherited in [false, true] {
        let mut command = command();
        let overrides: BTreeMap<_, _> = CAPABILITY_DEFAULTS
            .into_iter()
            .map(|(key, _)| (key, EXPLICIT_VALUE))
            .chain([(LC_CTYPE_ENV, EXPLICIT_VALUE), (CUSTOM_ENV, CUSTOM_VALUE)])
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect();
        if inherited {
            for key in overrides.keys() {
                command.env(key, INHERITED_VALUE);
            }
        }
        configure(&mut command, &overrides);
        for (key, value) in &overrides {
            assert_eq!(command.get_env(key), Some(OsStr::new(value)));
        }
    }
}

#[test]
fn inherited_capabilities_survive_when_no_override_is_supplied() {
    let mut command = command();
    for (key, _) in CAPABILITY_DEFAULTS {
        command.env(key, INHERITED_VALUE);
    }
    configure(&mut command, &BTreeMap::new());
    for (key, _) in CAPABILITY_DEFAULTS {
        assert_eq!(command.get_env(key), Some(OsStr::new(INHERITED_VALUE)));
    }
}

#[test]
fn cross_key_precedence_is_left_to_posix_even_with_launch_overrides() {
    let mut command = command();
    command.env(LC_ALL_ENV, "C");
    command.env(LC_CTYPE_ENV, "POSIX");
    let overrides = [(LANG_ENV.to_owned(), DEFAULT_CHARACTER_LOCALE.to_owned())].into();
    configure(&mut command, &overrides);
    assert_eq!(command.get_env(LC_ALL_ENV), Some(OsStr::new("C")));
    assert_eq!(command.get_env(LC_CTYPE_ENV), Some(OsStr::new("POSIX")));
    assert_eq!(
        command.get_env(LANG_ENV),
        Some(OsStr::new(DEFAULT_CHARACTER_LOCALE))
    );
}

#[test]
fn empty_launch_value_masks_inherited_value_before_default_resolution() {
    let mut command = command();
    command.env(LC_ALL_ENV, "C");
    command.env(TERM_ENV, INHERITED_VALUE);
    let overrides = [
        (LC_ALL_ENV.to_owned(), String::new()),
        (TERM_ENV.to_owned(), String::new()),
    ]
    .into();
    configure(&mut command, &overrides);
    assert_eq!(command.get_env(LC_ALL_ENV), Some(OsStr::new("")));
    assert_eq!(
        command.get_env(LC_CTYPE_ENV),
        Some(OsStr::new(DEFAULT_CHARACTER_LOCALE))
    );
    assert_eq!(
        command.get_env(TERM_ENV),
        Some(OsStr::new(CAPABILITY_DEFAULTS[0].1))
    );
}

#[test]
fn non_unicode_os_values_are_never_mistaken_for_missing_environment() {
    let value = OsString::from_vec(NON_UNICODE_VALUE.to_vec());
    for key in CHARACTER_LOCALE_KEYS
        .into_iter()
        .chain(CAPABILITY_DEFAULTS.map(|(key, _)| key))
    {
        let mut command = command();
        command.env(key, &value);
        configure(&mut command, &BTreeMap::new());
        assert_eq!(command.get_env(key), Some(value.as_os_str()));
        if [LC_ALL_ENV, LANG_ENV].contains(&key) {
            assert_eq!(command.get_env(LC_CTYPE_ENV), None);
        }
    }
}
