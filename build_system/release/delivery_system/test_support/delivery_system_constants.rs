pub const TRUE_EXECUTABLE: &str = "/usr/bin/true";
pub const CONTENTS_DIRECTORY_NAME: &str = "Contents";
pub const MACOS_DIRECTORY_NAME: &str = "MacOS";
pub const INFO_PLIST_NAME: &str = "Info.plist";
pub const TEST_EXECUTABLE_NAME: &str = "Yolo";
pub const FIXTURE_DIRECTORY_MODE: u32 = 0o755;
pub const FIXTURE_RESOURCE_MODE: u32 = 0o644;
pub const FIXTURE_EXECUTABLE_MODE: u32 = 0o755;
pub const ARCHIVE_PATH_SEPARATOR: &str = "/";
pub const CODESIGN_FORCE_ARGUMENT: &str = "--force";
pub const CODESIGN_SIGN_ARGUMENT: &str = "--sign";
pub const CODESIGN_AD_HOC_IDENTITY: &str = "-";
pub const CODESIGN_NO_TIMESTAMP_ARGUMENT: &str = "--timestamp=none";
pub const CODESIGN_SIGN_ARGUMENTS: &[&str] = &[
    CODESIGN_FORCE_ARGUMENT,
    CODESIGN_SIGN_ARGUMENT,
    CODESIGN_AD_HOC_IDENTITY,
    CODESIGN_NO_TIMESTAMP_ARGUMENT,
];
pub const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
<key>CFBundleExecutable</key><string>Yolo</string>
<key>CFBundleIdentifier</key><string>dev.yolo.delivery-test</string>
<key>CFBundlePackageType</key><string>APPL</string>
</dict>
</plist>
"#;
