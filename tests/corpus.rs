//! The real-world parser corpus, gathered from the test suites of fifteen
//! independent `FatturaPA` implementations, where their licences allow
//! redistribution. The licences, the attribution and the outcome of every
//! file are recorded in `tests/corpus/README.md`.
//!
//! Every file that validates against its official `XSD` is read, written,
//! validated again and re-read equal. Every file that does not validate is
//! listed in [`EXPECTED_XSD_FAILURES`] with the reason `xmllint` reports.
//! The signed `.p7m` files go through the signed reader, which unwraps the
//! `CMS` structure without the trust anchors, and the unwrapped content
//! goes through the same pipeline.
//!
//! Every readable invoice also goes through `validate` or
//! `validate_simplified` without panicking. The files on which the checks
//! report violations are listed in [`EXPECTED_VALIDATE_FINDINGS`]; every
//! reported violation is a true SDI finding about the file, checked against
//! the Elenco Controlli v2.0 text.
//!
//! The invoices that carry a `Signature` element round-trip equal, but the
//! written bytes are not XSD-validated, because the upstream `xsd-parser`
//! loses the `ds:` namespace of the element on serialisation
//! (Bergmann89/xsd-parser#308).

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

#[path = "support/mod.rs"]
mod support;

use std::fs;
use std::path::Path;

use fatturapa::signed;
use fatturapa::validate::{validate, validate_simplified};
use fatturapa::{
    AttestazioneTrasmissioneFattura, Document, FatturaElettronica, FatturaElettronicaSemplificata,
    MetadatiInvioFile, NotificaDecorrenzaTermini, NotificaEsito, NotificaEsitoCommittente,
    NotificaMancataConsegna, NotificaScarto, RicevutaConsegna, ScartoEsitoCommittente,
};

/// The corpus directory, relative to the crate root.
const CORPUS_DIR: &str = "tests/corpus";

/// The corpus files whose unwrapping is expected to fail, with a stable
/// part of the reason the signed reader reports.
const EXPECTED_UNWRAP_FAILURES: &[(&str, &str)] = &[
    (
        "fatturaelettronica-net/IT02182030391_31_tampered.Base64.xml.p7m",
        "not a CMS structure",
    ),
    (
        "fatturaelettronica-net/IT02182030391_31_tampered.xml.p7m",
        "not a CMS structure",
    ),
    (
        "fatturaelettronica-php/invalid.xml.p7m",
        "not a CMS structure",
    ),
];

/// The corpus files whose `XSD` validation is expected to fail, with a
/// stable part of the reason `xmllint` reports.
const EXPECTED_XSD_FAILURES: &[(&str, &str)] = &[
    (
        "fatturaelettronica-net/IT01234567890_FPA02.xml",
        "The value '10.0' is not accepted by the pattern",
    ),
    (
        "fatturaelettronica-net/IT01234567890_FPR02.xml",
        "The value '10.0' is not accepted by the pattern",
    ),
    (
        "fatturaelettronica-net/IT02182030391_32.xml",
        "Element 'DatiRitenuta': This element is not expected",
    ),
    (
        "fatturaelettronica-net/IT02182030391_32.windows-1252.xml",
        "Element 'DatiRitenuta': This element is not expected",
    ),
    (
        "fatturaelettronica-php/IT01234567890_11001.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "fatturaelettronica-php/IT01234567890_11001_entity.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "fatturaelettronica-php/IT01234567890_11001_multi_ritenute.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "fatturaelettronica-php/IT01234567890_11001_payment_details.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "fatturaelettronica-php/IT01234567890_11001_reso.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "fatturaelettronica-php/IT01234567890_11001_slash.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "fatturaelettronica-php/IT01234567890_11001_spazi.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "fatturaelettronica-php/IT01234567890_11002.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "fatturaelettronica-php/IT01234567890_21101.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "fatturapa-php-sdk/IT01234567890_FPA02.xml",
        "is not a valid value of the atomic type 'xs:date'",
    ),
    (
        "gobl-sdi/fatturapa.gobl/acube_test.xml",
        "Element 'CodiceDestinatario': This element is not expected",
    ),
    (
        "gobl-sdi/fatturapa.gobl/invoice-fund-contribution-mixed-retention.xml",
        "Element 'IstitutoFinanziario': This element is not expected",
    ),
    (
        "gobl-sdi/fatturapa.gobl/invoice-windows1252.xml",
        "Element 'CodiceArticolo': Missing child element(s)",
    ),
    (
        "json2fatturapaxml/IT03469550986_PR002.xml",
        "Element 'Nome': This element is not expected. Expected is one of ( Titolo, CodEORI )",
    ),
    (
        "marco-mastella/xmlInvalid/IT01234567890_FPA02.xml",
        "The value 'FPC12' is not an element of the set",
    ),
    (
        "marco-mastella/xmlInvalid/IT01234567890_FPR02.xml",
        "Element 'FatturaElettronicaBody': This element is not expected",
    ),
    (
        "marco-mastella/xmlInvalid/IT01234567890_FPR03.xml",
        "Missing child element(s). Expected is ( FatturaElettronicaBody )",
    ),
    (
        "mcp-fatturapa/invalid.xml",
        "Premature end of data in tag IdTrasmittente",
    ),
    (
        "mcp-fatturapa/valid_v13.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "openfatture/NS_2025-003.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "openfatture/RC_2025-002.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "phax-fatturapa/1.1/IT01234567890_11001.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "phax-fatturapa/1.1/IT01234567890_11002.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "phax-fatturapa/1.1/IT01234567890_11003.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "phax-fatturapa/1.1/IT01234567890_11004.xml",
        "No matching global declaration available for the validation root",
    ),
    (
        "phax-fatturapa/120/bad/IT01234567890_FPA01.xml",
        "Missing child element(s). Expected is ( FatturaElettronicaBody )",
    ),
    (
        "python-a38/dati_trasporto.xml",
        "Element 'DatiRitenuta': This element is not expected",
    ),
    ("python-a38/unicode.xml", "Missing child element(s)"),
    (
        "simevo-json/invalid/malformed.xml",
        "Namespace prefix p on FatturaElettronica is not defined",
    ),
    (
        "simevo-json/invalid/missing_CedentePrestatore.xml",
        "Element 'CessionarioCommittente': This element is not expected",
    ),
    (
        "simevo-json/invalid/missing_CessionarioCommittente.xml",
        "Element 'FatturaElettronicaHeader': Missing child element(s)",
    ),
    (
        "simevo-json/invalid/missing_CodiceDestinatario.xml",
        "Element 'ContattiTrasmittente': This element is not expected",
    ),
    (
        "simevo-json/invalid/missing_DatiBeniServizi.xml",
        "Element 'DatiPagamento': This element is not expected",
    ),
    (
        "simevo-json/invalid/missing_DatiGenerali.xml",
        "Element 'DatiBeniServizi': This element is not expected",
    ),
    (
        "simevo-json/invalid/missing_DatiGeneraliDocumento.xml",
        "Element 'DatiOrdineAcquisto': This element is not expected",
    ),
    (
        "simevo-json/invalid/missing_DatiGeneraliDocumento_Data.xml",
        "Element 'Numero': This element is not expected. Expected is ( Data )",
    ),
    (
        "simevo-json/invalid/missing_DatiGeneraliDocumento_Divisa.xml",
        "Element 'Data': This element is not expected",
    ),
    (
        "simevo-json/invalid/missing_DatiGeneraliDocumento_Numero.xml",
        "Element 'Causale': This element is not expected",
    ),
    (
        "simevo-json/invalid/missing_DatiGeneraliDocumento_TipoDocumento.xml",
        "Element 'Divisa': This element is not expected",
    ),
    (
        "simevo-json/invalid/missing_DatiTrasmissione.xml",
        "Element 'CedentePrestatore': This element is not expected",
    ),
    (
        "simevo-json/invalid/missing_FatturaElettronicaBody.xml",
        "Missing child element(s). Expected is ( FatturaElettronicaBody )",
    ),
    (
        "simevo-json/invalid/missing_FatturaElettronicaHeader.xml",
        "Element 'FatturaElettronicaBody': This element is not expected",
    ),
    (
        "simevo-json/invalid/missing_FormatoTrasmissione.xml",
        "Element 'CodiceDestinatario': This element is not expected. Expected is ( FormatoTrasmissione )",
    ),
    (
        "simevo-json/invalid/missing_IdTrasmittente.xml",
        "Element 'ProgressivoInvio': This element is not expected",
    ),
    (
        "simevo-json/invalid/missing_IdTrasmittente_IdCodice.xml",
        "Element 'IdTrasmittente': Missing child element(s)",
    ),
    (
        "simevo-json/invalid/missing_IdTrasmittente_IdPaese.xml",
        "Element 'IdCodice': This element is not expected",
    ),
    (
        "simevo-json/invalid/missing_ProgressivoInvio.xml",
        "Element 'CodiceDestinatario': This element is not expected. Expected is ( ProgressivoInvio )",
    ),
    (
        "simevo-json/random/00.xml",
        "Element 'Nome': This element is not expected. Expected is one of ( Titolo, CodEORI )",
    ),
    (
        "simevo-json/random/01.xml",
        "Element 'Cognome': This element is not expected. Expected is one of ( Titolo, CodEORI )",
    ),
    (
        "simevo-json/random/02.xml",
        "Element 'Cognome': This element is not expected. Expected is one of ( Titolo, CodEORI )",
    ),
    (
        "simevo-json/random/03.xml",
        "Element 'Cognome': This element is not expected. Expected is one of ( Titolo, CodEORI )",
    ),
    (
        "simevo-json/random/04.xml",
        "Element 'Cognome': This element is not expected. Expected is one of ( Titolo, CodEORI )",
    ),
    (
        "simevo-json/random/05.xml",
        "Element 'Cognome': This element is not expected. Expected is one of ( Titolo, CodEORI )",
    ),
    (
        "simevo-json/random/06.xml",
        "Element 'Nome': This element is not expected. Expected is one of ( Titolo, CodEORI )",
    ),
    (
        "simevo-json/random/07.xml",
        "Element 'Nome': This element is not expected. Expected is one of ( Titolo, CodEORI )",
    ),
    (
        "simevo-json/random/08.xml",
        "Element 'Cognome': This element is not expected. Expected is one of ( Titolo, CodEORI )",
    ),
    (
        "simevo-json/random/09.xml",
        "The value 'RF82' is not an element of the set",
    ),
    (
        "slamdunk/invalid_IT01234567890_11111_EC_001.xml",
        "is not a valid value of the atomic type",
    ),
    (
        "slamdunk/invalid_xml_tags.xml",
        "Namespace prefix ns2 on FatturaElettronica is not defined",
    ),
    (
        "slamdunk/invalid_xsd_content.xml",
        "The value 'ITALIA' is not accepted by the pattern '[A-Z]{2}'",
    ),
    (
        "slamdunk/invalid_xsd_structure_compliance.xml",
        "Element 'IdTrasmittente': Missing child element(s)",
    ),
    (
        "zugferd-corpus/eigor/valid/con-ritenuta-acconto-e-cassa-previdenziale.xml",
        "Premature end of data in tag FatturaElettronica",
    ),
    (
        "zugferd-corpus/official/valid/IT01234567890_FPR02.xml",
        "Element 'ContattiTrasmittente': This element is not expected",
    ),
];

/// The readable invoices on which `validate` or `validate_simplified` is
/// expected to report violations, as (code, path) pairs, every one a true
/// SDI finding checked against the Elenco Controlli v2.0 text. Every
/// readable invoice that is not listed here is clean.
const EXPECTED_VALIDATE_FINDINGS: &[(&str, &[(&str, &str)])] = &[
    (
        "fatturaelettronica-php/IT01234567899_000sq.xml",
        &[
            (
                "00421",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo[1]/Imposta",
            ),
            (
                "00421",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo[3]/Imposta",
            ),
            (
                "00445",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo[2]/Natura",
            ),
            (
                "00445",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DettaglioLinee[2]/Natura",
            ),
        ],
    ),
    (
        "gobl-sdi/fatturapa.gobl/invoice-b2g.xml",
        &[(
            "00427",
            "FatturaElettronicaHeader/DatiTrasmissione/CodiceDestinatario",
        )],
    ),
    (
        "gobl-sdi/gobl.fatturapa/out/invoice-b2g.xml",
        &[(
            "00427",
            "FatturaElettronicaHeader/DatiTrasmissione/CodiceDestinatario",
        )],
    ),
    (
        "gobl-sdi/gobl.fatturapa/out/invoice-simple-with-credits.xml",
        &[(
            "00423",
            "FatturaElettronicaBody[1]/DatiBeniServizi/DettaglioLinee[2]/PrezzoTotale",
        )],
    ),
    (
        "horstoeko-invoicesuite/02_technical_xml_fatturapa.xml",
        &[(
            "00423",
            "FatturaElettronicaBody[1]/DatiBeniServizi/DettaglioLinee[1]/PrezzoTotale",
        )],
    ),
    (
        "kamerozkan/rejected_arithmetic_fpr12.xml",
        &[
            (
                "00421",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo[1]/Imposta",
            ),
            (
                "00422",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo",
            ),
        ],
    ),
    (
        "marco-mastella/xmlInvoices/IT01234567890_FPR03.xml",
        &[
            (
                "00421",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo[1]/Imposta",
            ),
            (
                "00422",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo",
            ),
        ],
    ),
    (
        "phax-fatturapa/122/bad/IT01234567890_FPR01_TD29_RF20.xml",
        &[(
            "00475",
            "FatturaElettronicaBody[1]/DatiGenerali/DatiGeneraliDocumento/TipoDocumento",
        )],
    ),
    (
        "phax-fatturapa/122/good/IT01234567890_FPR02.xml",
        &[
            (
                "00421",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo[1]/Imposta",
            ),
            (
                "00422",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo",
            ),
        ],
    ),
    (
        "siboxd-model/IT01234567890_FPR03.xml",
        &[
            (
                "00421",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo[1]/Imposta",
            ),
            (
                "00422",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo",
            ),
        ],
    ),
    (
        "siboxd-model/IT01234567890_FPR04.xml",
        &[
            (
                "00421",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo[1]/Imposta",
            ),
            (
                "00422",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo",
            ),
        ],
    ),
    (
        "simevo-json/samples/IT01234567890_FPR03.xml",
        &[
            (
                "00421",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo[1]/Imposta",
            ),
            (
                "00422",
                "FatturaElettronicaBody[1]/DatiBeniServizi/DatiRiepilogo",
            ),
        ],
    ),
    (
        "zugferd-corpus/eigor/valid/A10-Licenses-CreditNote.xml",
        &[(
            "00476",
            "FatturaElettronicaHeader/CedentePrestatore/DatiAnagrafici/IdFiscaleIVA/IdPaese",
        )],
    ),
    (
        "zugferd-corpus/eigor/valid/A10-Licenses.xml",
        &[(
            "00476",
            "FatturaElettronicaHeader/CedentePrestatore/DatiAnagrafici/IdFiscaleIVA/IdPaese",
        )],
    ),
    (
        "zugferd-corpus/eigor/valid/A4-Subscription.xml",
        &[(
            "00476",
            "FatturaElettronicaHeader/CedentePrestatore/DatiAnagrafici/IdFiscaleIVA/IdPaese",
        )],
    ),
    (
        "zugferd-corpus/eigor/valid/fattpa-creditnote.to-ublcn-working-example.xml",
        &[(
            "00476",
            "FatturaElettronicaHeader/CedentePrestatore/DatiAnagrafici/IdFiscaleIVA/IdPaese",
        )],
    ),
];

/// Every corpus file validates as the curated lists say, round-trips equal,
/// and runs through the `validate` checks as the findings table says.
#[test]
fn corpus_round_trips() {
    let files = corpus_files(Path::new(CORPUS_DIR));
    assert!(!files.is_empty(), "the corpus is checked in");
    let mut failures: Vec<(String, String)> = Vec::new();
    let mut findings: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut valid = 0;
    let mut signed_docs = 0;
    for path in files {
        let relative = path
            .strip_prefix(CORPUS_DIR)
            .expect("the file is under the corpus dir")
            .to_string_lossy()
            .to_string();
        let bytes = fs::read(&path).expect("the corpus file is checked in");
        let content = if path.extension().is_some_and(|ext| ext == "p7m") {
            match signed::unwrap(&bytes) {
                Ok(content) => content,
                Err(error) => {
                    let expected = EXPECTED_UNWRAP_FAILURES
                        .iter()
                        .find(|(expected_path, _)| expected_path == &relative)
                        .unwrap_or_else(|| panic!("{relative} unwraps as it must"));
                    assert!(
                        error.to_string().contains(expected.1),
                        "{relative} fails to unwrap with an unexpected reason {error}"
                    );
                    eprintln!("{relative} fails to unwrap as expected");
                    continue;
                }
            }
        } else {
            bytes
        };
        let head = String::from_utf8_lossy(&content[..content.len().min(600)]);
        let root = root_name(&head);
        let family = family_for(&root);
        let schema = schema_for(family);
        match support::validate(Path::new(schema), &content) {
            Ok(()) => {
                match family {
                    Family::Ordinary => round_trip_ordinary(
                        &content,
                        schema,
                        &relative,
                        &mut findings,
                        &mut signed_docs,
                    ),
                    Family::Simplified => round_trip_simplified(
                        &content,
                        schema,
                        &relative,
                        &mut findings,
                        &mut signed_docs,
                    ),
                    Family::Sdi => match root.as_str() {
                        "RicevutaConsegna" => round_trip_sdi::<RicevutaConsegna>(&content, schema),
                        "NotificaScarto" => round_trip_sdi::<NotificaScarto>(&content, schema),
                        "NotificaMancataConsegna" => {
                            round_trip_sdi::<NotificaMancataConsegna>(&content, schema);
                        }
                        "NotificaEsito" => round_trip_sdi::<NotificaEsito>(&content, schema),
                        "MetadatiInvioFile" => {
                            round_trip_sdi::<MetadatiInvioFile>(&content, schema);
                        }
                        "NotificaEsitoCommittente" => {
                            round_trip_sdi::<NotificaEsitoCommittente>(&content, schema);
                        }
                        "ScartoEsitoCommittente" => {
                            round_trip_sdi::<ScartoEsitoCommittente>(&content, schema);
                        }
                        "NotificaDecorrenzaTermini" => {
                            round_trip_sdi::<NotificaDecorrenzaTermini>(&content, schema);
                        }
                        "AttestazioneTrasmissioneFattura" => {
                            round_trip_sdi::<AttestazioneTrasmissioneFattura>(&content, schema);
                        }
                        other => {
                            panic!("{relative} is an SDI message with the unknown root {other}")
                        }
                    },
                }
                valid += 1;
            }
            Err(reason) => failures.push((relative, reason)),
        }
    }
    assert_expectations(&failures, &findings);
    assert!(valid > 0, "the valid files round-trip");
    eprintln!(
        "{valid} corpus files round-trip, {} fail as expected, {} signed documents round-trip without the written XSD validation",
        failures.len(),
        signed_docs
    );
}

/// The assertions of the corpus test, on the collected failures and
/// findings.
fn assert_expectations(
    failures: &[(String, String)],
    findings: &[(String, Vec<(String, String)>)],
) {
    assert_eq!(
        failures.len(),
        EXPECTED_XSD_FAILURES.len(),
        "the failures are {failures:?}"
    );
    let mut mismatches: Vec<String> = Vec::new();
    for (path, reason) in failures {
        let Some(expected) = EXPECTED_XSD_FAILURES
            .iter()
            .find(|(expected_path, _)| expected_path == path)
        else {
            mismatches.push(format!("{path} fails the XSD validation unexpectedly"));
            continue;
        };
        if !reason.contains(expected.1) {
            mismatches.push(format!("{path} fails with an unexpected reason {reason}"));
        }
    }
    for (path, expected) in EXPECTED_VALIDATE_FINDINGS {
        let Some(actual) = findings.iter().find(|(actual_path, _)| actual_path == path) else {
            mismatches.push(format!("{path} must be readable to report the findings"));
            continue;
        };
        if finding_set(&actual.1) != finding_set(expected) {
            mismatches.push(format!("{path} reports unexpected findings {:?}", actual.1));
        }
    }
    for (path, found) in findings {
        let listed = EXPECTED_VALIDATE_FINDINGS
            .iter()
            .any(|(listed_path, _)| listed_path == path);
        if !listed && !found.is_empty() {
            mismatches.push(format!("{path} is clean but reports {found:?}"));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// The family of the document by its root element name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    /// The ordinary invoice, the `FPR12` and `FPA12` schema.
    Ordinary,
    /// The simplified invoice, the `FSM10` schema.
    Simplified,
    /// An SDI message, the `MessaggiTypes` schema.
    Sdi,
}

/// The schema file of the family.
fn schema_for(family: Family) -> &'static str {
    match family {
        Family::Ordinary => "spec/Schema_VFPR12_v1.2.3.xsd",
        Family::Simplified => "spec/Schema_VFSM10v_1.0.2.xsd",
        Family::Sdi => "spec/MessaggiTypes_v1.1.xsd",
    }
}

/// The family of the document by its root element name.
fn family_for(root: &str) -> Family {
    if root == "FatturaElettronicaSemplificata" {
        Family::Simplified
    } else if matches!(
        root,
        "RicevutaConsegna"
            | "NotificaScarto"
            | "NotificaMancataConsegna"
            | "NotificaEsito"
            | "MetadatiInvioFile"
            | "NotificaEsitoCommittente"
            | "ScartoEsitoCommittente"
            | "NotificaDecorrenzaTermini"
            | "AttestazioneTrasmissioneFattura"
    ) {
        Family::Sdi
    } else {
        Family::Ordinary
    }
}

/// The root element name, without its prefix, of the document in `head`.
fn root_name(head: &str) -> String {
    head.split('<')
        .find_map(|rest| {
            let tag = rest.trim_start();
            if tag.is_empty() || tag.starts_with('!') || tag.starts_with('?') {
                return None;
            }
            let tag = tag.split('>').next()?.split(' ').next()?;
            if tag.is_empty() || tag.starts_with('!') || tag.starts_with('?') {
                return None;
            }
            Some(tag.split(':').next_back()?.to_string())
        })
        .unwrap_or_default()
}

/// Whether `content` carries a `Signature` element, whose serialisation
/// waits for the upstream `xsd-parser` namespace fix.
fn carries_signature(content: &[u8]) -> bool {
    content.windows(13).any(|w| w == b"<ds:Signature")
        || content.windows(10).any(|w| w == b"<Signature")
}

/// The `(code, path)` pairs of `found`, as a set, independent of the order
/// `validate` reports them.
fn finding_set<T>(found: &[(T, T)]) -> Vec<(String, String)>
where
    T: AsRef<str>,
{
    let mut set: Vec<(String, String)> = found
        .iter()
        .map(|(code, path)| (code.as_ref().to_string(), path.as_ref().to_string()))
        .collect();
    set.sort();
    set
}

/// Reads `content` as an ordinary invoice, writes it, re-reads it equal, and
/// records the `validate` findings. The written bytes are XSD-validated,
/// unless the document carries a `Signature` element, which the upstream
/// `xsd-parser` defect keeps unqualified on serialisation.
fn round_trip_ordinary(
    content: &[u8],
    schema: &str,
    relative: &str,
    findings: &mut Vec<(String, Vec<(String, String)>)>,
    signed_docs: &mut usize,
) {
    let parsed = FatturaElettronica::from_xml_slice(content).expect("the invoice reads");
    let written = parsed.to_xml_vec().expect("the invoice writes");
    if carries_signature(content) {
        *signed_docs += 1;
    } else {
        support::validate(Path::new(schema), &written).expect("the written invoice validates");
    }
    let reread =
        FatturaElettronica::from_xml_slice(&written).expect("the written invoice re-reads");
    assert_eq!(parsed, reread, "{relative} round-trips");
    let found = validate(&parsed)
        .into_iter()
        .map(|violation| (violation.code, violation.path))
        .collect();
    findings.push((relative.to_string(), found));
}

/// The simplified invoice variant of `round_trip_ordinary`.
fn round_trip_simplified(
    content: &[u8],
    schema: &str,
    relative: &str,
    findings: &mut Vec<(String, Vec<(String, String)>)>,
    signed_docs: &mut usize,
) {
    let parsed = FatturaElettronicaSemplificata::from_xml_slice(content)
        .expect("the simplified invoice reads");
    let written = parsed.to_xml_vec().expect("the simplified invoice writes");
    if carries_signature(content) {
        *signed_docs += 1;
    } else {
        support::validate(Path::new(schema), &written)
            .expect("the written simplified invoice validates");
    }
    let reread = FatturaElettronicaSemplificata::from_xml_slice(&written)
        .expect("the written simplified invoice re-reads");
    assert_eq!(parsed, reread, "{relative} round-trips");
    let found = validate_simplified(&parsed)
        .into_iter()
        .map(|violation| (violation.code, violation.path))
        .collect();
    findings.push((relative.to_string(), found));
}

/// Reads the SDI message `content` as `T`, writes it, validates the written
/// bytes against the SDI message schema, re-reads them and checks equality
/// with the first read.
fn round_trip_sdi<T>(content: &[u8], schema: &str)
where
    T: Document + std::fmt::Debug + PartialEq,
{
    let parsed = T::from_xml_slice(content).expect("the message reads");
    let written = parsed.to_xml_vec().expect("the message writes");
    support::validate(Path::new(schema), &written).expect("the written message validates");
    let reread = T::from_xml_slice(&written).expect("the written message re-reads");
    assert_eq!(parsed, reread, "the message round-trips");
}

/// Every corpus file, sorted, recursing into the source directories.
fn corpus_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).expect("the corpus dir is checked in") {
        let path = entry.expect("the read_dir entry is readable").path();
        if path.is_dir() {
            out.extend(corpus_files(&path));
        } else {
            let is_corpus = path
                .extension()
                .is_some_and(|ext| matches!(ext.to_str(), Some("xml" | "p7m")));
            if is_corpus {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}
