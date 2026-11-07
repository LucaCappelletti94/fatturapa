//! Round-trip property tests: an arbitrary document of every root type is
//! written, XSD-validated where the upstream `xsd-parser` `ds:Signature`
//! defect does not block it, re-read, and compared with the original. The
//! generators live in the `generator` module, which the fuzz round-trip
//! target shares.
//!
//! The seven SDI message types that require a `ds:Signature` in the schema
//! cannot be XSD-validated after writing until the upstream fix lands, so
//! their properties write, re-read and compare, mirroring the `read_stable`
//! sample tests.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

#[path = "support/mod.rs"]
mod support;

#[path = "generator/mod.rs"]
mod generator;

use fatturapa::Document;
use fatturapa::{
    AttestazioneTrasmissioneFattura, FatturaElettronica, FatturaElettronicaSemplificata,
    MetadatiInvioFile, NotificaDecorrenzaTermini, NotificaEsito, NotificaEsitoCommittente,
    NotificaMancataConsegna, NotificaScarto, RicevutaConsegna, ScartoEsitoCommittente,
};
use proptest::prelude::*;
use proptest::property_test;

/// A random ordinary invoice writes, validates against
/// `Schema_VFPR12_v1.2.3.xsd`, re-reads and compares equal.
#[property_test(config = ProptestConfig {
    cases: 64,
    ..ProptestConfig::default()
})]
fn round_trip_fattura_elettronica(seed: u64) {
    let doc = generator::invoice_from_seed(seed);
    let bytes = doc.to_xml_vec().expect("the invoice serialises");
    support::validate(&support::fpr12_schema(), &bytes).expect("the invoice validates");
    let reread = FatturaElettronica::from_xml_slice(&bytes).expect("the invoice reads");
    assert_eq!(doc, reread);
}

/// A random simplified invoice writes, validates against
/// `Schema_VFSM10v_1.0.2.xsd`, re-reads and compares equal.
#[property_test(config = ProptestConfig {
    cases: 64,
    ..ProptestConfig::default()
})]
fn round_trip_fattura_elettronica_semplificata(seed: u64) {
    let doc = generator::semplificata_from_seed(seed);
    let bytes = doc.to_xml_vec().expect("the invoice serialises");
    support::validate(&support::fsm10_schema(), &bytes).expect("the invoice validates");
    let reread = FatturaElettronicaSemplificata::from_xml_slice(&bytes).expect("the invoice reads");
    assert_eq!(doc, reread);
}

/// A random `MT` message writes, validates against
/// `MessaggiTypes_v1.1.xsd`, re-reads and compares equal.
#[property_test(config = ProptestConfig {
    cases: 64,
    ..ProptestConfig::default()
})]
fn round_trip_metadati_invio_file(seed: u64) {
    let doc = generator::mt_from_seed(seed);
    let bytes = doc.to_xml_vec().expect("the message serialises");
    support::validate(&support::sdi_schema(), &bytes).expect("the message validates");
    let reread = MetadatiInvioFile::from_xml_slice(&bytes).expect("the message reads");
    assert_eq!(doc, reread);
}

/// A random `EC` message without a signature writes, validates against
/// `MessaggiTypes_v1.1.xsd`, re-reads and compares equal.
#[property_test(config = ProptestConfig {
    cases: 64,
    ..ProptestConfig::default()
})]
fn round_trip_notifica_esito_committente(seed: u64) {
    let doc = generator::ec_from_seed(seed);
    let bytes = doc.to_xml_vec().expect("the message serialises");
    support::validate(&support::sdi_schema(), &bytes).expect("the message validates");
    let reread = NotificaEsitoCommittente::from_xml_slice(&bytes).expect("the message reads");
    assert_eq!(doc, reread);
}

/// A random `AT` message writes, re-reads and compares equal. The XSD
/// validation of the written bytes waits for the upstream `xsd-parser`
/// `ds:Signature` namespace fix.
#[property_test(config = ProptestConfig {
    cases: 64,
    ..ProptestConfig::default()
})]
fn round_trip_attestazione_trasmissione_fattura(seed: u64) {
    let doc = generator::at_from_seed(seed);
    let bytes = doc.to_xml_vec().expect("the message serialises");
    let reread =
        AttestazioneTrasmissioneFattura::from_xml_slice(&bytes).expect("the message reads");
    assert_eq!(doc, reread);
}

/// A random `DT` message writes, re-reads and compares equal. The XSD
/// validation of the written bytes waits for the upstream `xsd-parser`
/// `ds:Signature` namespace fix.
#[property_test(config = ProptestConfig {
    cases: 64,
    ..ProptestConfig::default()
})]
fn round_trip_notifica_decorrenza_termini(seed: u64) {
    let doc = generator::dt_from_seed(seed);
    let bytes = doc.to_xml_vec().expect("the message serialises");
    let reread = NotificaDecorrenzaTermini::from_xml_slice(&bytes).expect("the message reads");
    assert_eq!(doc, reread);
}

/// A random `MC` message writes, re-reads and compares equal. The XSD
/// validation of the written bytes waits for the upstream `xsd-parser`
/// `ds:Signature` namespace fix.
#[property_test(config = ProptestConfig {
    cases: 64,
    ..ProptestConfig::default()
})]
fn round_trip_notifica_mancata_consegna(seed: u64) {
    let doc = generator::mc_from_seed(seed);
    let bytes = doc.to_xml_vec().expect("the message serialises");
    let reread = NotificaMancataConsegna::from_xml_slice(&bytes).expect("the message reads");
    assert_eq!(doc, reread);
}

/// A random `NE` message writes, re-reads and compares equal. The XSD
/// validation of the written bytes waits for the upstream `xsd-parser`
/// `ds:Signature` namespace fix.
#[property_test(config = ProptestConfig {
    cases: 64,
    ..ProptestConfig::default()
})]
fn round_trip_notifica_esito(seed: u64) {
    let doc = generator::ne_from_seed(seed);
    let bytes = doc.to_xml_vec().expect("the message serialises");
    let reread = NotificaEsito::from_xml_slice(&bytes).expect("the message reads");
    assert_eq!(doc, reread);
}

/// A random `NS` message writes, re-reads and compares equal. The XSD
/// validation of the written bytes waits for the upstream `xsd-parser`
/// `ds:Signature` namespace fix.
#[property_test(config = ProptestConfig {
    cases: 64,
    ..ProptestConfig::default()
})]
fn round_trip_notifica_scarto(seed: u64) {
    let doc = generator::ns_from_seed(seed);
    let bytes = doc.to_xml_vec().expect("the message serialises");
    let reread = NotificaScarto::from_xml_slice(&bytes).expect("the message reads");
    assert_eq!(doc, reread);
}

/// A random `RC` message writes, re-reads and compares equal. The XSD
/// validation of the written bytes waits for the upstream `xsd-parser`
/// `ds:Signature` namespace fix.
#[property_test(config = ProptestConfig {
    cases: 64,
    ..ProptestConfig::default()
})]
fn round_trip_ricevuta_consegna(seed: u64) {
    let doc = generator::rc_from_seed(seed);
    let bytes = doc.to_xml_vec().expect("the message serialises");
    let reread = RicevutaConsegna::from_xml_slice(&bytes).expect("the message reads");
    assert_eq!(doc, reread);
}

/// A random `SE` message writes, re-reads and compares equal. The XSD
/// validation of the written bytes waits for the upstream `xsd-parser`
/// `ds:Signature` namespace fix.
#[property_test(config = ProptestConfig {
    cases: 64,
    ..ProptestConfig::default()
})]
fn round_trip_scarto_esito_committente(seed: u64) {
    let doc = generator::se_from_seed(seed);
    let bytes = doc.to_xml_vec().expect("the message serialises");
    let reread = ScartoEsitoCommittente::from_xml_slice(&bytes).expect("the message reads");
    assert_eq!(doc, reread);
}

/// The official sample corpus is checked in and non-empty, so the sample
/// tests have their input.
#[test]
fn sample_corpus_is_checked_in() {
    const FILES: [&str; 14] = [
        "IT01234567890_FPA01.xml",
        "IT01234567890_FPA02.xml",
        "IT01234567890_FPA03.xml",
        "IT01234567890_FPR01.xml",
        "IT01234567890_FPR02.xml",
        "IT01234567890_FPR03.xml",
        "IT01234567890_11111_AT_001.xml",
        "IT01234567890_11111_DT_001.xml",
        "IT01234567890_11111_EC_001.xml",
        "IT01234567890_11111_MC_001.xml",
        "IT01234567890_11111_MT_001.xml",
        "IT01234567890_11111_NE_001.xml",
        "IT01234567890_11111_NS_001.xml",
        "IT01234567890_11111_RC_001.xml",
    ];
    for file in FILES {
        assert!(
            !support::sample_bytes(file).is_empty(),
            "{file} is checked in"
        );
    }
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

/// The `IdentificativoSdI` is an `xsd:integer` of twelve digits, so an
/// identifier outside the `i32` range reads, writes, re-reads and compares
/// equal.
#[test]
fn sdi_identifier_round_trips_twelve_digits() {
    let xml = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <types:NotificaEsitoCommittente \
        xmlns:types=\"http://www.fatturapa.gov.it/sdi/messaggi/v1.0\" versione=\"1.0\">\
        <IdentificativoSdI>990000000012</IdentificativoSdI>\
        <Esito>EC01</Esito>\
        </types:NotificaEsitoCommittente>";
    let doc = NotificaEsitoCommittente::from_xml_slice(xml).expect("the message reads");
    let bytes = doc.to_xml_vec().expect("the message serialises");
    support::validate(&support::sdi_schema(), &bytes).expect("the message validates");
    let reread =
        NotificaEsitoCommittente::from_xml_slice(&bytes).expect("the written message re-reads");
    assert_eq!(doc, reread);
    assert!(
        bytes.windows(12).any(|window| window == b"990000000012"),
        "the written message keeps the twelve-digit identifier"
    );
}
