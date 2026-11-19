//! The messages of `MessaggiFatturaTypes_v1.0.xsd`, which the SDI exchanges
//! for invoices between private parties: read, checked field by field,
//! written, XSD-validated, re-read, equal.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

#[path = "support/mod.rs"]
mod support;

use std::fmt::Debug;

use fatturapa::generated::messaggi_fattura::{
    FileMetadati, RicevutaConsegna, RicevutaImpossibilitaRecapito, RicevutaScarto,
};
use fatturapa::{Date, DateTime, Document};

/// The SHA-256 the fixtures carry, of the official sample
/// `IT01234567890_FPR01.xml` with `LF` line ends.
const HASH: &str = "bcfabace608de074f7987904684a27245feb9488f21e2e8455284eefa973ff66";

/// Reads the fixture `file` from `tests/fixtures/messaggi-fattura` after
/// validating it against the schema, writes it, validates the written bytes,
/// re-reads them and returns the first read once both reads are equal.
fn round_trips<T: Document + Debug + PartialEq>(file: &str) -> T {
    let schema = support::messaggi_fattura_schema();
    let bytes = support::fixture_bytes(&format!("messaggi-fattura/{file}"));
    support::validate(&schema, &bytes).expect("the fixture validates against the schema");
    let doc = T::from_xml_slice(&bytes).expect("the fixture reads");
    let written = doc.to_xml_vec().expect("the message serialises");
    support::validate(&schema, &written)
        .expect("the written message validates against the official schema");
    let reread = T::from_xml_slice(&written).expect("the written message reads");
    assert_eq!(doc, reread);
    doc
}

/// The metadata the SDI attaches to every invoice it delivers carries the
/// `Hash` of the invoice file between `NomeFile` and `CodiceDestinatario`.
#[test]
fn file_metadati_round_trips() {
    let doc: FileMetadati = round_trips("IT01234567890_FPR01_MT_001.xml");
    let content = &doc.content;
    assert_eq!(content.identificativo_sd_i, "99000000001");
    assert_eq!(content.nome_file, "IT01234567890_FPR01.xml");
    assert_eq!(content.hash, HASH);
    assert_eq!(content.codice_destinatario, "0000000");
    assert_eq!(content.formato, "FPR12");
    assert_eq!(content.tentativi_invio, 1);
    assert_eq!(content.message_id, "99000000002");
    assert_eq!(content.note, None);
}

/// The delivery receipt carries the `Hash`, the `FlussoSemplificato`
/// attribute and the recipient the file reached.
#[test]
fn ricevuta_consegna_round_trips() {
    let doc: RicevutaConsegna = round_trips("IT01234567890_FPR01_RC_001.xml");
    assert_eq!(doc.flusso_semplificato.as_deref(), Some("Si"));
    let content = &doc.content;
    assert_eq!(content.hash, HASH);
    assert_eq!(
        content.data_ora_consegna,
        "2026-11-19T17:52:59.000+01:00"
            .parse::<DateTime>()
            .expect("a valid dateTime")
    );
    assert_eq!(content.destinatario.content.codice, "ABC1234");
    assert_eq!(
        content.pec_message_id.as_deref(),
        Some("99000000003.0000000000000@pec.fatturapa.it")
    );
}

/// The rejection receipt carries every error with its `Suggerimento`.
#[test]
fn ricevuta_scarto_round_trips() {
    let doc: RicevutaScarto = round_trips("IT01234567890_FPR01_NS_001.xml");
    let errors = &doc.content.lista_errori.content.errore;
    let codes: Vec<&str> = errors
        .iter()
        .map(|error| error.content.codice.as_str())
        .collect();
    assert_eq!(codes, ["00305", "00423"]);
    assert!(
        errors[1]
            .content
            .suggerimento
            .starts_with("Verificare che PrezzoTotale")
    );
    assert_eq!(doc.content.hash, HASH);
}

/// The non-delivery receipt carries the date the file was made available to
/// the recipient and the archive the file came in.
#[test]
fn ricevuta_impossibilita_recapito_round_trips() {
    let doc: RicevutaImpossibilitaRecapito = round_trips("IT01234567890_FPR01_MC_001.xml");
    let content = &doc.content;
    assert_eq!(
        content.data_messa_a_disposizione,
        "2026-11-21".parse::<Date>().expect("a valid date")
    );
    let archive = content
        .riferimento_archivio
        .as_ref()
        .expect("the archive reference");
    assert_eq!(archive.content.nome_file, "IT01234567890_FPR01.zip");
    assert_eq!(content.hash, HASH);
}
