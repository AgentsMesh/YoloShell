use super::*;
#[path = "../test_support/serde_constants.rs"]
mod constants;
use constants::RENAME_STYLE;

const FIRST_LINE: usize = 1;
const BYTE_ADAPTER: &str = SERDE_BYTES_ADAPTER;
const UNRELATED_VALUE: &str = "custom";

#[test]
fn only_rename_style_and_exact_bytes_with_adapter_are_metadata_exemptions() {
    for (source, value, exempt) in [
        ("#[serde(rename_all = \"snake_case\")]", RENAME_STYLE, true),
        ("#[serde(with = \"serde_bytes\")]", BYTE_ADAPTER, true),
        ("#[serde(rename = \"serde_bytes\")]", BYTE_ADAPTER, false),
        ("#[serde(with = \"custom\")]", UNRELATED_VALUE, false),
        ("#[serde(rename_all = \"custom\")]", RENAME_STYLE, false),
    ] {
        assert_eq!(
            matches!(
                metadata_disposition(source, FIRST_LINE, value, false),
                MetadataDisposition::Exempt
            ),
            exempt,
            "{source}"
        );
    }
}

#[test]
fn closed_attributes_do_not_exempt_literals_on_later_lines() {
    let source = "#[serde(with = \"serde_bytes\")]\nconst VALUE: &str = \"serde_bytes\";";
    assert!(matches!(
        metadata_disposition(source, FIRST_LINE + FIRST_LINE, BYTE_ADAPTER, false),
        MetadataDisposition::Ordinary
    ));
    let multiline = "#[serde(\nwith = \"serde_bytes\"\n)]";
    assert!(matches!(
        metadata_disposition(multiline, FIRST_LINE + FIRST_LINE, BYTE_ADAPTER, false),
        MetadataDisposition::Exempt
    ));
}
