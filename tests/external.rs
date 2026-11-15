//! Tests over `tests/fixtures/external` and `tests/corpus`, which stay in the repository only and out of the published crate.
#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

use fatturapa::signed::{ChainError, SignedError, unwrap, verify};
use x509_cert::Certificate;

// Outcome tags follow the match arms of the two tests below, files as recorded in `tests/fixtures/external/README.md`.

/// The `DER` of the `CMS` in `input`, mirroring the normalisation of the
/// `signed` module.
fn external_cms_der(input: &[u8]) -> Option<Vec<u8>> {
    use base64::Engine;
    use cms::content_info::ContentInfo;
    use der::Decode;
    use rasn_cms::{ContentInfo as RasnContentInfo, SignedData as RasnSignedData};

    if ContentInfo::from_der(input).is_ok() {
        return Some(input.to_vec());
    }
    let stripped: Vec<u8> = input
        .iter()
        .copied()
        .filter(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        .collect();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&stripped)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(&stripped))
        .ok()?;
    if ContentInfo::from_der(&decoded).is_ok() {
        return Some(decoded);
    }
    let content_info = rasn::ber::decode::<RasnContentInfo>(input).ok()?;
    let signed_data = rasn::ber::decode::<RasnSignedData>(content_info.content.as_bytes()).ok()?;
    let signed_data_der = rasn::der::encode(&signed_data).ok()?;
    let fresh = RasnContentInfo {
        content_type: content_info.content_type,
        content: rasn::types::Any::new(signed_data_der),
    };
    rasn::der::encode(&fresh).ok()
}

/// The `CMS` certificates embedded in `input`, when the structure parses.
fn external_embedded_certs(input: &[u8]) -> Vec<Certificate> {
    use cms::cert::CertificateChoices;
    use cms::content_info::ContentInfo;
    use cms::signed_data::SignedData;
    use const_oid::db;
    use der::{Decode, Encode};

    let Some(der) = external_cms_der(input) else {
        return Vec::new();
    };
    let content_info = ContentInfo::from_der(&der).expect("the content info decodes");
    if content_info.content_type != db::rfc5911::ID_SIGNED_DATA {
        return Vec::new();
    }
    let signed_data_der = content_info.content.to_der().expect("the content encodes");
    let signed_data = SignedData::from_der(&signed_data_der).expect("the signed data decodes");
    let mut certs = Vec::new();
    if let Some(set) = &signed_data.certificates {
        for choice in set.0.iter() {
            if let CertificateChoices::Certificate(cert) = choice {
                certs.push(cert.clone());
            }
        }
    }
    certs
}

const UNWRAP_OUTCOMES: &[(&str, u8, &str)] = &[
    (
        "tests/fixtures/external/dss/cades/validation/CAdESDoubleLTA.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-ats-v3-rev-val-crl.p7s",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-ats-v3-wrong-cert.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-ats-v3-wrong-crl.p7s",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-b-copied-cnttst.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-bes-signeddata-detached.p7s",
        2,
        "1.2.840.113549.1.7.2",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-bes-signeddata-enveloping.p7m",
        2,
        "1.2.840.113549.1.7.2",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-broken-sig-tst-and-ats-v3.cms",
        0,
        "4",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-broken-sig-tst.p7m",
        0,
        "4",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-dss1956.p7m",
        0,
        "36",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-dss1997.p7m",
        0,
        "74827",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-extended-a.pkcs7",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-extended-bes.pkcs7",
        0,
        "11",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-extended-epes-sig-pol-store.pkcs7",
        0,
        "11",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-extended-epes.pkcs7",
        0,
        "11",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-level-b-sig-policy-invalid-digest-algo.p7m",
        0,
        "11",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-lta-copied-atst.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-lta-duplicated-atst.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-lta-en319122.cms",
        0,
        "4",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-lta-ts101733.cms",
        0,
        "4",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-t-copied-sigtst.p7m",
        0,
        "11",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-t-duplicated-sigtst.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-two-sig-copied-tst.p7m",
        0,
        "13",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades_signeddata_extended_after_lta.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cms-no-sign-cert.p7m",
        0,
        "12",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cms-wrong-sign-cert-hash-algo.pkcs7",
        0,
        "11",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cms-wrong-sign-cert-ref.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/counterSig.p7m",
        0,
        "3",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/counterSignedLTA.p7s",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1220/CAdES-BpB_revoked_signingCertificate.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1220/CAdES-BpLTA_modified_ats_hash_element.p7m",
        3,
        "the input is not a CMS structure",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1220/CAdES-BpLTA_removed_ocsp.p7m",
        3,
        "the input is not a CMS structure",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1220/CAdES-BpT_modified_ts_hash.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1344/screenshot.png.signed_qes_detached.p7s",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA256-SHA3_256withRSA.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA256-SHA3_256withRSAandMGF1.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA256-SHA512withRSA.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_224-SHA256withRSA.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_224-SHA3_224withRSA.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_256-SHA256withRSA.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_256-SHA3_256withRSA.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_384-SHA256withRSA.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_512-SHA256withRSA.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_512-SHA3_512withRSA.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1469/cadesLTAwithATv2expired.p7s",
        0,
        "74683",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1670/signatureExtendedTwoLTA.p7s",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-2011/cades-content-tst.pkcs7",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-2011/cades-detached.pkcs7",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-2011/cades-enveloping-broken.pkcs7",
        0,
        "74827",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-2011/cades-enveloping.pkcs7",
        0,
        "11",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-2011/cades-lta-detached.pkcs7",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-2011/cades-tstv2-detached.p7s",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-916/test.txt.signed_Certipost-2048.detached.old.p7s",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-B-B-basic-der.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-B-B-wrong-er-attribute.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS-basic-invalid-sig.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS-detached-ber.p7s",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS-detached-invalid.p7s",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS-detached-original-not-covered.p7s",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS-detached.p7s",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS-detached_renewed_hashtree.p7s",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS-detached_renewed_hashtree_invalid.p7s",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS-parallel-ers.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS-two-er-mixed.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS-two-er.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS_renewed_hashtree.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/C-E-ERS_renewed_hashtree_invalid.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/CAdES-BpT.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/CAdEs-BpT+ER-without-reducedHashTree.p7m",
        0,
        "20",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/Double-C-E-ERS-detached_hashtree_renewal.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/Double-C-E-ERS.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/Signature-C-LT-d233a2d9-a257-40dc-bcdb-bf4516b6d1da.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/Signature-C-LTA-d233a2d9-a257-40dc-bcdb-bf4516b6d1da.p7m",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/malformed-cades.p7m",
        3,
        "the input is not a CMS structure",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/no-key-info-cert.pkcs7",
        0,
        "11",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/wrongContentHints.p7m",
        0,
        "10377",
    ),
    (
        "tests/fixtures/external/dss/cms/timestamp.tst",
        2,
        "1.2.840.113549.1.9.16.1.4",
    ),
    (
        "tests/fixtures/external/dss/spi/archive_timestamp.tst",
        2,
        "1.2.840.113549.1.9.16.1.4",
    ),
    (
        "tests/fixtures/external/dss/spi/disig.tst",
        2,
        "1.2.840.113549.1.9.16.1.4",
    ),
    (
        "tests/fixtures/external/dss/spi/tst_with_broken_ocsp.tst",
        3,
        "the CMS SignedData structure is malformed",
    ),
    (
        "tests/fixtures/external/dss/validation/tst-two-refs.tst",
        2,
        "1.2.840.113549.1.9.16.1.4",
    ),
];

const VERIFY_OUTCOMES: &[(&str, u8, &str)] = &[
    (
        "tests/fixtures/external/dss/cades/validation/cades-broken-sig-tst-and-ats-v3.cms",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-broken-sig-tst.p7m",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-dss1956.p7m",
        10,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-dss1997.p7m",
        8,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-extended-bes.pkcs7",
        7,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-extended-epes-sig-pol-store.pkcs7",
        7,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-extended-epes.pkcs7",
        7,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-level-b-sig-policy-invalid-digest-algo.p7m",
        0,
        "2",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-lta-en319122.cms",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-lta-ts101733.cms",
        1,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-t-copied-sigtst.p7m",
        0,
        "2",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cades-two-sig-copied-tst.p7m",
        0,
        "2",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cms-no-sign-cert.p7m",
        5,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/cms-wrong-sign-cert-hash-algo.pkcs7",
        4,
        "1.3.14.3.2.26",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/counterSig.p7m",
        5,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1220/CAdES-BpB_revoked_signingCertificate.p7m",
        8,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1220/CAdES-BpT_modified_ts_hash.p7m",
        9,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA256-SHA3_256withRSA.p7m",
        4,
        "2.16.840.1.101.3.4.3.14",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA256-SHA3_256withRSAandMGF1.p7m",
        4,
        "1.2.840.113549.1.1.10",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA256-SHA512withRSA.p7m",
        3,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_224-SHA256withRSA.p7m",
        4,
        "2.16.840.1.101.3.4.2.7",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_224-SHA3_224withRSA.p7m",
        4,
        "2.16.840.1.101.3.4.2.7",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_256-SHA256withRSA.p7m",
        4,
        "2.16.840.1.101.3.4.2.8",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_256-SHA3_256withRSA.p7m",
        4,
        "2.16.840.1.101.3.4.2.8",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_384-SHA256withRSA.p7m",
        4,
        "2.16.840.1.101.3.4.2.9",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_512-SHA256withRSA.p7m",
        4,
        "2.16.840.1.101.3.4.2.10",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1419/CAdES-BpB-att-SHA3_512-SHA3_512withRSA.p7m",
        4,
        "2.16.840.1.101.3.4.2.10",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-1469/cadesLTAwithATv2expired.p7s",
        10,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-2011/cades-enveloping-broken.pkcs7",
        2,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/dss-2011/cades-enveloping.pkcs7",
        0,
        "2",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/CAdES-BpT.p7m",
        8,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/evidence-record/CAdEs-BpT+ER-without-reducedHashTree.p7m",
        8,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/no-key-info-cert.pkcs7",
        6,
        "",
    ),
    (
        "tests/fixtures/external/dss/cades/validation/wrongContentHints.p7m",
        4,
        "1.2.840.113549.1.1.10",
    ),
];

/// Every external file `unwrap`s to the recorded content or error.
#[test]
fn external_unwrap_outcomes_are_pinned() {
    for (path, expected_tag, expected_detail) in UNWRAP_OUTCOMES {
        let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("{path}: {error}"));
        let (tag, detail) = match unwrap(&bytes) {
            Ok(content) => (0, content.len().to_string()),
            Err(SignedError::DetachedContent) => (1, String::new()),
            Err(SignedError::UnsupportedContentType(oid)) => (2, oid),
            Err(SignedError::NotCms(reason)) => (3, reason),
            Err(other) => panic!("{path}: the unexpected error {other:?}"),
        };
        assert_eq!(
            (tag, detail),
            (*expected_tag, expected_detail.to_string()),
            "{path}"
        );
    }
}

/// Every external file that `unwrap`s to content `verify`es to the recorded
/// outcome with its embedded certificates as the anchors.
#[test]
fn external_verify_outcomes_are_pinned() {
    for (path, expected_tag, expected_detail) in VERIFY_OUTCOMES {
        let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("{path}: {error}"));
        let certs = external_embedded_certs(&bytes);
        let (tag, detail) = match verify(&bytes, &certs, None) {
            Ok(verified) => (0, verified.chain().len().to_string()),
            Err(SignedError::SignatureInvalid) => (1, String::new()),
            Err(SignedError::DigestMismatch) => (2, String::new()),
            Err(SignedError::AlgorithmMismatch) => (3, String::new()),
            Err(SignedError::UnsupportedAlgorithm(oid)) => (4, oid),
            Err(SignedError::UnsupportedSignerIdentifier) => (5, String::new()),
            Err(SignedError::SignerCertificateNotFound) => (6, String::new()),
            Err(SignedError::MissingSigningTime) => (7, String::new()),
            Err(SignedError::Chain(ChainError::IssuerNotInStore)) => (8, String::new()),
            Err(SignedError::Chain(ChainError::ExtendedKeyUsage)) => (9, String::new()),
            Err(SignedError::Chain(ChainError::UnsupportedAlgorithm(_))) => (10, String::new()),
            Err(other) => panic!("{path}: the unexpected error {other:?}"),
        };
        assert_eq!(
            (tag, detail),
            (*expected_tag, expected_detail.to_string()),
            "{path}"
        );
    }
}

/// The trusted list snapshots and the real signatures verified against the
/// `AgID` list.
#[cfg(feature = "trust-list")]
mod trust_list {
    use std::fs;

    use fatturapa::signed::verify;
    use fatturapa::trust_list::{TrustListError, trust_anchors};

    /// The Actalis qualified signature of the `fatturaelettronica-php`
    /// corpus verifies against the `AgID` list, although its `sid` writes
    /// the issuer name in `PrintableString` where the certificate uses
    /// `UTF8String`.
    #[test]
    fn actalis_signature_verifies_against_the_agid_list() {
        let anchors =
            trust_anchors(&fs::read("tests/fixtures/TSL-IT.xml").expect("the list is readable"))
                .expect("the list parses");
        let bytes = fs::read("tests/corpus/fatturaelettronica-php/IT00484960588_ERKHK.xml.p7m")
            .expect("the signed file is readable");
        let verified = verify(&bytes, &anchors, None).expect("the qualified signature verifies");
        assert_eq!(verified.chain().len(), 2);
        assert_eq!(verified.signing_time().to_string(), "2018-12-14T10:21:38Z");
    }

    /// The historical `AgID` snapshots of `tests/fixtures/external`, one per
    /// year from 2017 to 2024, with the anchor count each one parses to.
    const EXTERNAL_TSL_SNAPSHOTS: &[(&str, usize)] = &[
        (
            "tests/fixtures/external/tsl-it/TSL-IT-20170610215159.xml",
            113,
        ),
        (
            "tests/fixtures/external/tsl-it/TSL-IT-20211021145639.xml",
            145,
        ),
        (
            "tests/fixtures/external/tsl-it/TSL-IT-20220121051644.xml",
            143,
        ),
        (
            "tests/fixtures/external/tsl-it/TSL-IT-20231230095405.xml",
            143,
        ),
        (
            "tests/fixtures/external/tsl-it/TSL-IT-20241224070933.xml",
            138,
        ),
    ];

    /// Every historical snapshot parses to the recorded anchor count.
    #[test]
    fn historical_snapshots_parse_to_the_recorded_anchors() {
        for (path, expected) in EXTERNAL_TSL_SNAPSHOTS {
            let bytes = fs::read(path).expect("the snapshot is readable");
            let anchors = trust_anchors(&bytes).expect("the snapshot parses");
            assert_eq!(anchors.len(), *expected, "{path}");
        }
    }

    /// The truncated 2019 snapshot is a list cut off before the root element
    /// closes, which must fail.
    #[test]
    fn truncated_snapshot_is_rejected() {
        let bytes = fs::read("tests/fixtures/external/tsl-it/TSL-IT-20190824181858-truncated.xml")
            .expect("the snapshot is readable");
        let err = trust_anchors(&bytes).expect_err("the root element is unclosed");
        assert!(matches!(err, TrustListError::NotTrustedList), "{err:?}");
    }

    /// The `EU` list of trusted lists parses to zero trust anchors, because it
    /// lists trusted lists rather than qualified certificate authorities.
    #[test]
    fn eu_lotl_parses_to_zero_anchors() {
        let bytes =
            fs::read("tests/fixtures/external/lotl/eu-lotl.xml").expect("the list is readable");
        let anchors = trust_anchors(&bytes).expect("the list parses");
        assert!(anchors.is_empty(), "the LOTL carries no trust anchors");
    }
}
