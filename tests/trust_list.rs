//! Trust anchors from the ETSI trusted services list, the `trust-list`
//! feature.
#![cfg(feature = "trust-list")]
#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

use std::fs;

use der::DecodePem;
use der::Encode;
use fatturapa::trust_list::{TrustListError, trust_anchors};
use fatturapa::{Document, FatturaElettronica};
use x509_cert::Certificate;

/// The bytes of the `AgID` trusted services list fixture.
fn tsl() -> Vec<u8> {
    fs::read("tests/fixtures/TSL-IT.xml").expect("the fixture is readable")
}

/// The current `AgID` list parses to the granted qualified CA anchors.
#[test]
fn agid_list_parses_to_anchors() {
    let anchors = trust_anchors(&tsl()).expect("the list parses");
    assert_eq!(
        anchors.len(),
        142,
        "every granted QC CA service is an anchor"
    );
    for anchor in &anchors {
        assert!(
            anchor.tbs_certificate.extensions.is_some(),
            "a trust anchor carries its extensions"
        );
    }
}

/// The Banca d'Italia qualified CA of the list is present.
#[test]
fn banca_d_italia_is_present() {
    let anchors = trust_anchors(&tsl()).expect("the list parses");
    let mut found = false;
    for anchor in &anchors {
        let subject = anchor
            .tbs_certificate
            .subject
            .to_der()
            .expect("the subject encodes");
        if subject
            .windows(14)
            .any(|window| window == b"Banca d'Italia")
        {
            found = true;
        }
    }
    assert!(found, "the Banca d'Italia anchor is in the list");
}

/// An `XML` document that is not a trusted list is rejected.
#[test]
fn non_trusted_list_is_rejected() {
    let sample = fs::read("spec/samples/IT01234567890_FPR01.xml").expect("the sample is readable");
    let invoice = FatturaElettronica::from_xml_slice(&sample).expect("the sample parses");
    let mut xml = Vec::new();
    invoice.to_writer(&mut xml).expect("the invoice writes");
    assert!(
        matches!(trust_anchors(&xml), Err(TrustListError::NotTrustedList)),
        "an invoice is not a trusted list"
    );
}

/// A truncated list is rejected as malformed `XML`.
#[test]
fn truncated_list_is_rejected() {
    let list = tsl();
    let half = list.len() / 2;
    assert!(matches!(
        trust_anchors(&list[..half]),
        Err(TrustListError::Xml(_) | TrustListError::NotTrustedList)
    ));
}

/// A granted service of a type other than qualified certificate authority, a
/// withdrawn qualified certificate authority, and services whose type or
/// certificate sits only inside a `ServiceHistory` entry, are not trust anchors.
#[test]
fn synthetic_list_keeps_only_the_granted_qc_ca() {
    let tsl = fs::read("tests/fixtures/tsl-synthetic.xml").expect("the fixture is readable");
    let anchors = trust_anchors(&tsl).expect("the list parses");
    assert_eq!(
        anchors.len(),
        1,
        "the granted QC CA service is the only anchor"
    );
    let ca_pem = fs::read("tests/fixtures/ca.pem").expect("the fixture is readable");
    let ca = Certificate::from_pem(ca_pem.as_slice()).expect("the fixture is a PEM certificate");
    assert_eq!(
        anchors[0].tbs_certificate.subject,
        ca.tbs_certificate.subject
    );
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
    let bytes = fs::read("tests/fixtures/external/lotl/eu-lotl.xml").expect("the list is readable");
    let anchors = trust_anchors(&bytes).expect("the list parses");
    assert!(anchors.is_empty(), "the LOTL carries no trust anchors");
}
