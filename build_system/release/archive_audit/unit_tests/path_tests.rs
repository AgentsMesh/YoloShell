use yolo_release_archive_audit::constants::ROOT_DIRECTORY_ENTRY;
use yolo_release_archive_audit_test_support::{FixtureEntry, assert_rejected, build};

const SAFE_PATH: &[u8] = b"Yolo.app/Contents/payload";

fn archive_with_name(name: &[u8]) -> Vec<u8> {
    build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        FixtureEntry::regular(name),
    ])
    .bytes
}

#[test]
fn unsafe_ascii_paths_are_rejected() {
    let paths: &[&[u8]] = &[
        b"Other.app/payload",
        b"/Yolo.app/payload",
        b"Yolo.app\\payload",
        b"Yolo.app//payload",
        b"Yolo.app/./payload",
        b"Yolo.app/../payload",
        b"Yolo.app/Contents/",
        b"Yolo.app/Contents/payload.",
        b"Yolo.app/Contents/payload ",
        b"Yolo.app/Contents/pay:load",
        b"Yolo.app/Contents/pay\0load",
        b"Yolo.app/Contents/pay\nload",
    ];
    for path in paths {
        assert_rejected(&archive_with_name(path));
    }
}

#[test]
fn unicode_nfc_nfd_long_s_and_emoji_are_rejected() {
    let paths: &[&[u8]] = &[
        b"Yolo.app/Contents/\xc3\xa9",
        b"Yolo.app/Contents/e\xcc\x81",
        b"Yolo.app/Contents/\xc5\xbf",
        b"Yolo.app/Contents/\xf0\x9f\xa4\x96",
    ];
    for path in paths {
        assert_rejected(&archive_with_name(path));
    }
}

#[test]
fn raw_duplicate_and_case_folded_aliases_are_rejected() {
    let duplicate = build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        FixtureEntry::regular(SAFE_PATH),
        FixtureEntry::regular(SAFE_PATH),
    ]);
    assert_rejected(&duplicate.bytes);
    let case_alias = build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        FixtureEntry::regular(b"Yolo.app/Contents/Agent"),
        FixtureEntry::regular(b"Yolo.app/Contents/agent"),
    ]);
    assert_rejected(&case_alias.bytes);
    let file_directory_alias = build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        FixtureEntry::regular(b"Yolo.app/Contents"),
        FixtureEntry::directory(b"Yolo.app/Contents/"),
    ]);
    assert_rejected(&file_directory_alias.bytes);
}

#[test]
fn root_file_and_missing_explicit_root_directory_are_rejected() {
    assert_rejected(&build(&[FixtureEntry::regular(b"Yolo.app")]).bytes);
    assert_rejected(&build(&[FixtureEntry::regular(SAFE_PATH)]).bytes);
}
