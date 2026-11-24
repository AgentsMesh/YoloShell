use crate::{
    constants::*,
    fixture::{manifest, read},
};
use yolo_coverage_source_contract::{DECLARATION_SOURCE_SETS, SourceContract, verify_manifest};

#[test]
fn reviewed_declarations_require_exact_source_bytes_with_lf_or_crlf_manifests() {
    for sources in DECLARATION_SOURCE_SETS {
        let manifest = manifest(sources);
        for value in [manifest.clone(), manifest.replace('\n', "\r\n")] {
            assert_eq!(
                verify_manifest(&value, read).unwrap(),
                SourceContract::Declarations
            );
        }
        assert!(
            verify_manifest(&manifest, |source| {
                let mut bytes = read(source)?;
                bytes.extend_from_slice(EXTRA_DEFINITION);
                Ok(bytes)
            })
            .is_err()
        );
        assert!(verify_manifest(&manifest, |_| Err("missing source".to_owned())).is_err());
    }
}

#[test]
fn unknown_missing_duplicate_and_traversing_production_sources_are_rejected() {
    for sources in DECLARATION_SOURCE_SETS {
        let valid = manifest(sources);
        let mut invalid_manifests = vec![
            String::new(),
            format!("{valid}\n{SOURCE}"),
            format!("{valid}\n{}", sources[FIRST_SOURCE].0),
            format!("{valid}\ncore/src/../missing.rs"),
            format!("{valid}\n/workspace/core/src/missing.rs"),
            format!("{valid}\nexternal/../build_system/missing/src/lib.rs"),
        ];
        if sources.len() > 1 {
            invalid_manifests.push(manifest(&sources[1..]));
        }
        for invalid in invalid_manifests {
            assert!(verify_manifest(&invalid, read).is_err(), "{invalid}");
        }
    }
}
