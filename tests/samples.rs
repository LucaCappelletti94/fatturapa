//! The official samples: read, write, XSD-validate, re-read, equal.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

#[path = "support/mod.rs"]
mod support;

use fatturapa::Document;
use fatturapa::{
    AttestazioneTrasmissioneFattura, FatturaElettronica, MetadatiInvioFile,
    NotificaDecorrenzaTermini, NotificaEsito, NotificaEsitoCommittente, NotificaMancataConsegna,
    NotificaScarto, RicevutaConsegna, ScartoEsitoCommittente,
};

/// Reads the invoice sample `file`, writes it, validates the written bytes
/// against the ordinary invoice schema, re-reads them and checks equality
/// with the first read.
fn round_trip_invoice(file: &str) {
    let doc = FatturaElettronica::from_xml_slice(&support::sample_bytes(file))
        .expect("the sample reads and re-reads");
    let bytes = doc.to_xml_vec().expect("the invoice serialises");
    support::validate(&support::fpr12_schema(), &bytes)
        .expect("the written invoice validates against the official schema");
    let reread = FatturaElettronica::from_xml_slice(&bytes).expect("the written invoice reads");
    assert_eq!(doc, reread);
}

/// Reads the SDI sample `file`, writes it, validates the written bytes
/// against the SDI message schema, re-reads them and checks equality with
/// the first read.
fn round_trip_sdi<T: Document + std::fmt::Debug + PartialEq>(file: &str) {
    let doc = T::from_xml_slice(&support::sample_bytes(file)).expect("the sample reads");
    let bytes = doc.to_xml_vec().expect("the message serialises");
    support::validate(&support::sdi_schema(), &bytes)
        .expect("the written message validates against the official schema");
    let reread = T::from_xml_slice(&bytes).expect("the written message reads");
    assert_eq!(doc, reread);
}

/// The `round_trip_sdi` variant for the signed samples: it reads the sample,
/// writes it and re-reads it equal, without XSD-validating the written bytes,
/// because writing a `ds:Signature` waits for the upstream `xsd-parser` fix.
fn read_stable_sdi<T: Document + std::fmt::Debug + PartialEq>(file: &str) {
    let doc = T::from_xml_slice(&support::sample_bytes(file)).expect("the sample reads");
    let bytes = doc.to_xml_vec().expect("the message serialises");
    let reread = T::from_xml_slice(&bytes).expect("the written message reads");
    assert_eq!(doc, reread);
}

/// `FPA01` is an ordinary invoice sample, the public administration flavour.
#[test]
fn sample_fpa01_round_trip() {
    round_trip_invoice("IT01234567890_FPA01.xml");
}

/// `FPA02` carries the simplified cedente anagrafica without an `Anagrafica`.
#[test]
fn sample_fpa02_round_trip() {
    round_trip_invoice("IT01234567890_FPA02.xml");
}

/// `FPA03` carries the simplified cessionario anagrafica without an `Anagrafica`.
#[test]
fn sample_fpa03_round_trip() {
    round_trip_invoice("IT01234567890_FPA03.xml");
}

/// `FPR01` is the private ordinary invoice sample.
#[test]
fn sample_fpr01_round_trip() {
    round_trip_invoice("IT01234567890_FPR01.xml");
}

/// `FPR03` carries the `SoggettoEmittente` block.
#[test]
fn sample_fpr03_round_trip() {
    round_trip_invoice("IT01234567890_FPR03.xml");
}

/// `from_reader` reads through a file and a cursor equal to `from_xml_slice`.
#[test]
fn from_reader_matches_from_xml_slice() {
    use std::io::Cursor;
    let file = "IT01234567890_FPR03.xml";
    let bytes = support::sample_bytes(file);
    let invoice = FatturaElettronica::from_xml_slice(&bytes).expect("the sample reads");
    assert_eq!(
        FatturaElettronica::from_reader(
            std::fs::File::open(support::sample(file)).expect("the sample is checked in")
        )
        .expect("the file reads"),
        invoice
    );
    assert_eq!(
        FatturaElettronica::from_reader(Cursor::new(bytes)).expect("the cursor reads"),
        invoice
    );

    let file = "IT01234567890_11111_RC_001.xml";
    let bytes = support::sample_bytes(file);
    let message = RicevutaConsegna::from_xml_slice(&bytes).expect("the sample reads");
    assert_eq!(
        RicevutaConsegna::from_reader(
            std::fs::File::open(support::sample(file)).expect("the sample is checked in")
        )
        .expect("the file reads"),
        message
    );
    assert_eq!(
        RicevutaConsegna::from_reader(Cursor::new(bytes)).expect("the cursor reads"),
        message
    );
}

/// `from_xml_str` and `to_xml_string` agree with the slice and vector
/// forms on the same document.
#[test]
fn str_and_string_forms_match_the_slice_forms() {
    let file = "IT01234567890_FPR03.xml";
    let bytes = support::sample_bytes(file);
    let text = String::from_utf8(bytes).expect("the sample is UTF-8");
    let invoice = FatturaElettronica::from_xml_slice(text.as_bytes()).expect("the sample reads");
    assert_eq!(
        FatturaElettronica::from_xml_str(&text).expect("the text reads"),
        invoice
    );
    let written = invoice.to_xml_vec().expect("the invoice serialises");
    assert_eq!(
        invoice.to_xml_string().expect("the invoice serialises"),
        String::from_utf8(written).expect("the written bytes are UTF-8")
    );
}

/// `FPR02` is officially invalid against `Schema_VFPR12_v1.2.3.xsd`, because
/// it places `PECDestinatario` before `ContattiTrasmittente` inside
/// `DatiTrasmissione`, and the generated deserialiser must reject it with an
/// unexpected-element error naming the offending element.
#[test]
fn sample_fpr02_rejected() {
    let err = FatturaElettronica::from_xml_slice(&support::sample_bytes("IT01234567890_FPR02.xml"))
        .expect_err("the sample violates the element order of 1.2.3");
    let text = err.to_string();
    assert!(
        text.contains("ContattiTrasmittente"),
        "the rejection names the offending element, got {text}"
    );
}

/// `MT` is an unsigned SDI sample with the full read, write, validate,
/// re-read cycle.
#[test]
fn sample_mt_round_trip() {
    round_trip_sdi::<MetadatiInvioFile>("IT01234567890_11111_MT_001.xml");
}

/// `EC` is an unsigned SDI sample with the full read, write, validate,
/// re-read cycle.
#[test]
fn sample_ec_round_trip() {
    round_trip_sdi::<NotificaEsitoCommittente>("IT01234567890_11111_EC_001.xml");
}

/// `AT` carries a `ds:Signature`. Reading and re-reading the written copy
/// must stay equal; the XSD validation waits for the upstream fix.
#[test]
fn sample_at_read_stable() {
    read_stable_sdi::<AttestazioneTrasmissioneFattura>("IT01234567890_11111_AT_001.xml");
}

/// `AT` written bytes validate against the SDI schema.
#[test]
#[ignore = "waits for the upstream xsd-parser fix for ds:Signature namespaces, see upstream/xsd-parser-signature-namespace.md"]
fn sample_at_written_validates() {
    round_trip_sdi::<AttestazioneTrasmissioneFattura>("IT01234567890_11111_AT_001.xml");
}

/// `DT` carries a `ds:Signature`. Reading and re-reading the written copy
/// must stay equal; the XSD validation waits for the upstream fix.
#[test]
fn sample_dt_read_stable() {
    read_stable_sdi::<NotificaDecorrenzaTermini>("IT01234567890_11111_DT_001.xml");
}

/// `DT` written bytes validate against the SDI schema.
#[test]
#[ignore = "waits for the upstream xsd-parser fix for ds:Signature namespaces, see upstream/xsd-parser-signature-namespace.md"]
fn sample_dt_written_validates() {
    round_trip_sdi::<NotificaDecorrenzaTermini>("IT01234567890_11111_DT_001.xml");
}

/// `MC` carries a `ds:Signature`. Reading and re-reading the written copy
/// must stay equal; the XSD validation waits for the upstream fix.
#[test]
fn sample_mc_read_stable() {
    read_stable_sdi::<NotificaMancataConsegna>("IT01234567890_11111_MC_001.xml");
}

/// `MC` written bytes validate against the SDI schema.
#[test]
#[ignore = "waits for the upstream xsd-parser fix for ds:Signature namespaces, see upstream/xsd-parser-signature-namespace.md"]
fn sample_mc_written_validates() {
    round_trip_sdi::<NotificaMancataConsegna>("IT01234567890_11111_MC_001.xml");
}

/// `NE` carries a `ds:Signature`. Reading and re-reading the written copy
/// must stay equal; the XSD validation waits for the upstream fix.
#[test]
fn sample_ne_read_stable() {
    read_stable_sdi::<NotificaEsito>("IT01234567890_11111_NE_001.xml");
}

/// `NE` written bytes validate against the SDI schema.
#[test]
#[ignore = "waits for the upstream xsd-parser fix for ds:Signature namespaces, see upstream/xsd-parser-signature-namespace.md"]
fn sample_ne_written_validates() {
    round_trip_sdi::<NotificaEsito>("IT01234567890_11111_NE_001.xml");
}

/// `NS` carries a `ds:Signature`. Reading and re-reading the written copy
/// must stay equal; the XSD validation waits for the upstream fix.
#[test]
fn sample_ns_read_stable() {
    read_stable_sdi::<NotificaScarto>("IT01234567890_11111_NS_001.xml");
}

/// `NS` written bytes validate against the SDI schema.
#[test]
#[ignore = "waits for the upstream xsd-parser fix for ds:Signature namespaces, see upstream/xsd-parser-signature-namespace.md"]
fn sample_ns_written_validates() {
    round_trip_sdi::<NotificaScarto>("IT01234567890_11111_NS_001.xml");
}

/// `RC` carries a `ds:Signature`. Reading and re-reading the written copy
/// must stay equal; the XSD validation waits for the upstream fix.
#[test]
fn sample_rc_read_stable() {
    read_stable_sdi::<RicevutaConsegna>("IT01234567890_11111_RC_001.xml");
}

/// `RC` written bytes validate against the SDI schema.
#[test]
#[ignore = "waits for the upstream xsd-parser fix for ds:Signature namespaces, see upstream/xsd-parser-signature-namespace.md"]
fn sample_rc_written_validates() {
    round_trip_sdi::<RicevutaConsegna>("IT01234567890_11111_RC_001.xml");
}

/// `SE` carries a `ds:Signature`. Reading and re-reading the written copy
/// must stay equal; the XSD validation waits for the upstream fix.
#[test]
fn sample_se_read_stable() {
    read_stable_sdi::<ScartoEsitoCommittente>("IT01234567890_11111_SE_001.xml");
}

/// `SE` written bytes validate against the SDI schema.
#[test]
#[ignore = "waits for the upstream xsd-parser fix for ds:Signature namespaces, see upstream/xsd-parser-signature-namespace.md"]
fn sample_se_written_validates() {
    round_trip_sdi::<ScartoEsitoCommittente>("IT01234567890_11111_SE_001.xml");
}

/// The schemas and the catalog the validation helper relies on are checked
/// in.
#[test]
fn spec_files_exist() {
    for path in [
        support::fpr12_schema(),
        support::fsm10_schema(),
        support::sdi_schema(),
        support::catalog(),
    ] {
        assert!(path.exists(), "{path:?} is checked in");
    }
}
