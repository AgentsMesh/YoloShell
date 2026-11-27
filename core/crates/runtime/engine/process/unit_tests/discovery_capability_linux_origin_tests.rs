use super::*;
use crate::process::PtySessionProof;

const EARLIER_BIRTH_TICKS: u64 = 1;

#[test]
fn generated_capability_and_clone_preserve_actual_issuer_birth() {
    let issuer = ProcessIdentity::capture(std::process::id()).unwrap();
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    assert_eq!(capability.platform.origin, Some(issuer));
    assert_eq!(capability.clone().platform.origin, Some(issuer));
}

#[test]
fn reconstructed_capability_never_acquires_an_issuer_boundary() {
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let reconstructed = LinuxDiscoveryCapability::from_value(capability.value()).unwrap();
    let adopted = LinuxDiscoveryCapability::adopted(capability.value()).unwrap();
    for restored in [reconstructed, adopted] {
        assert_eq!(restored.origin, None);
        assert_eq!(restored.clone().origin, None);
        assert_eq!(restored.reader.0, capability.reader_handle());
        assert_eq!(restored.writer.0, capability.writer_handle());
    }
}

#[test]
fn ordinary_capture_uses_stored_origin_before_capturing_the_process_tree() {
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let issuer = capability.platform.origin.unwrap();
    let older_root = ProcessIdentity {
        start_time: issuer.start_time.checked_sub(EARLIER_BIRTH_TICKS).unwrap(),
        ..issuer
    };
    let mut proof = PtySessionProof::discovery(capability.value().into()).unwrap();
    let origin_error = proof
        .bind_linux_capability_origin(Some(issuer), older_root)
        .unwrap_err()
        .to_string();
    for generated in [capability.clone(), capability] {
        let error = generated.capture_process_tree(older_root).err().unwrap();
        assert_eq!(error.to_string(), origin_error);

        let adopted = ProcessDiscoveryCapability {
            platform: LinuxDiscoveryCapability::adopted(generated.value()).unwrap(),
            value: generated.value().into(),
        };
        let error = adopted.capture_process_tree(older_root).err().unwrap();
        assert_ne!(error.to_string(), origin_error);
    }
}
