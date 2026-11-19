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

/// The error the reader reports on the official sample `file`.
fn sample_rejection<T: Document + Debug>(file: &str) -> String {
    T::from_xml_slice(&support::sample_bytes(file))
        .expect_err("the sample lacks an attribute the schema requires")
        .to_string()
}

/// The Agenzia publishes four of its examples of these messages without the
/// `versione` attribute the schema requires, and the reader refuses them,
/// naming the attribute.
#[test]
fn official_samples_without_versione_rejected() {
    for message in [
        sample_rejection::<FileMetadati>("ITHVQWPH73P42H501Y_00022_MT_001.xml"),
        sample_rejection::<RicevutaConsegna>("ITHVQWPH73P42H501Y_00022_RC_001.xml"),
        sample_rejection::<RicevutaScarto>("ITHVQWPH73P42H501Y_00022_NS_001.xml"),
        sample_rejection::<RicevutaImpossibilitaRecapito>("ITHVQWPH73P42H501Y_00022_MC_001.xml"),
    ] {
        assert!(
            message.contains("versione"),
            "the rejection names the attribute, got {message}"
        );
    }
}

/// The Agenzia example of a rejection receipt that carries `versione` reads
/// with its error, writes, validates and re-reads equal. The signatures are
/// compared with the in-scope namespaces of their `xs:any` content cleared,
/// because the example binds its namespaces to prefixes the writer does not
/// pick.
#[test]
fn official_ricevuta_scarto_sample_round_trips() {
    let mut doc = RicevutaScarto::from_xml_slice(&support::sample_bytes(
        "ITHVQWPH73P42H501Y_00022_NS__001.xml",
    ))
    .expect("the sample reads");
    let errors = &doc.content.lista_errori.content.errore;
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].content.codice, "00002");
    assert_eq!(errors[0].content.descrizione, "Nome file duplicato");
    assert_eq!(
        doc.content.hash,
        "ef912599ea43cddf0c9ee59e30e38b2a91f0793ef046e61aa120a9d3ee0b5992"
    );
    let written = doc.to_xml_vec().expect("the receipt serialises");
    support::validate(&support::messaggi_fattura_schema(), &written)
        .expect("the written receipt validates against the official schema");
    let mut reread = RicevutaScarto::from_xml_slice(&written).expect("the written receipt reads");
    doc.content.signature =
        support::normalize_messaggi_fattura_signature(doc.content.signature.clone());
    reread.content.signature =
        support::normalize_messaggi_fattura_signature(reread.content.signature.clone());
    assert_eq!(doc, reread);
}
