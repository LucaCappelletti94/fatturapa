//! The cross-border invoice fixtures, hand-written against the
//! specification. Each fixture is validated against the official schema,
//! read back, round-tripped and checked with `validate`, and each broken
//! copy raises the Elenco Controlli code it should.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use fatturapa::builder::{Invoice, InvoiceBody, Line};
use fatturapa::generated::fpr12 as g;
use fatturapa::validate::{Violation, validate};
use fatturapa::{Amount, Date, Document, FatturaElettronica};

fn amount(text: &str) -> Amount {
    text.parse().expect("a valid decimal")
}

fn date(year: i32, month: u32, day: u32) -> Date {
    format!("{year:04}-{month:02}-{day:02}")
        .parse()
        .expect("a valid date")
}

fn assert_has(violations: &[Violation], code: &str) {
    assert!(
        violations.iter().any(|v| v.code == code),
        "expected {code}, got {violations:?}"
    );
}

/// Reads the fixture `file` from `tests/fixtures/invoices`, validates the
/// checked-in bytes against the ordinary invoice schema, reads them,
/// writes them back, validates the written bytes and compares the re-read
/// with the first read.
fn fixture_round_trips(file: &str) -> FatturaElettronica {
    let path = Path::new("tests/fixtures/invoices").join(file);
    let bytes = std::fs::read(&path).expect("the fixture is checked in");
    support::validate(&support::fpr12_schema(), &bytes)
        .expect("the fixture validates against the schema");
    let doc = FatturaElettronica::from_xml_slice(&bytes).expect("the fixture reads");
    let written = doc.to_xml_vec().expect("the fixture serialises");
    support::validate(&support::fpr12_schema(), &written)
        .expect("the written fixture validates against the schema");
    let reread = FatturaElettronica::from_xml_slice(&written).expect("the fixture re-reads");
    assert_eq!(reread, doc, "the fixture round-trips equal");
    doc
}

fn seller() -> g::CedentePrestatoreType {
    g::CedentePrestatoreType::new(
        g::IdFiscaleType::new("IT", "01234567890"),
        g::AnagraficaType::denominazione("ALPHA SRL"),
        g::RegimeFiscaleType::Rf01,
        g::IndirizzoType::new("VIA ROMA 1", "00100", "ROMA", "IT"),
    )
}

/// The US business client of the art. 7-ter fixture, without an Italian
/// VAT number, so its `IdFiscaleIVA` carries the country of residence and
/// the fiscal identifier assigned there.
fn buyer_us() -> g::CessionarioCommittenteType {
    g::CessionarioCommittenteType::new(
        g::IdFiscaleType::new("US", "US123456789"),
        g::AnagraficaType::denominazione("ACME INC"),
        g::IndirizzoType::new("FIFTH AVENUE 1", "10001", "NEW YORK", "US"),
    )
}

/// The German business client of the art. 7-ter fixture.
fn buyer_de() -> g::CessionarioCommittenteType {
    g::CessionarioCommittenteType::new(
        g::IdFiscaleType::new("DE", "DE998877665"),
        g::AnagraficaType::denominazione("MUSTER GMBH"),
        g::IndirizzoType::new("BEISPIELSTRASSE 1", "10115", "BERLIN", "DE"),
    )
}

/// Fixture 1: services to a non-EU business client, the art. 7-ter case.
/// The line carries the `N2.1` nature at a zero rate, the
/// `RiferimentoNormativo` the specification requires when a `Natura` is
/// present, and the virtual stamp duty, the total being above 77.47 EUR.
#[test]
fn td01_services_to_a_non_eu_us_client() {
    let doc = fixture_round_trips("td01-7ter-non-eu-us.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    let header = &doc.content.fattura_elettronica_header.content;
    assert_eq!(
        header
            .dati_trasmissione
            .content
            .codice_destinatario
            .as_str(),
        "XXXXXXX"
    );
    let buyer = &header
        .cessionario_committente
        .content
        .dati_anagrafici
        .content;
    assert_eq!(
        buyer
            .id_fiscale_iva
            .as_ref()
            .expect("the buyer carries its VAT number")
            .content
            .id_paese,
        "US"
    );
    let line = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dettaglio_linee[0]
        .content;
    assert_eq!(line.aliquota_iva, amount("0.00"));
    assert_eq!(line.natura, Some(g::NaturaType::N21));
    let riep = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dati_riepilogo[0]
        .content;
    assert_eq!(
        riep.riferimento_normativo.as_deref(),
        Some("ART. 7-TER DPR 633/72")
    );
    let generali = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content;
    assert_eq!(
        generali
            .dati_bollo
            .as_ref()
            .expect("the stamp duty is present")
            .content
            .importo_bollo,
        Some(amount("2.00"))
    );
    assert_eq!(generali.importo_totale_documento, Some(amount("150.00")));
}

/// The same invoice built with the `Invoice` builder equals the hand-checked
/// fixture, pinning the builder to it.
#[test]
fn td01_non_eu_us_client_matches_the_builder() {
    let fixture = fixture_round_trips("td01-7ter-non-eu-us.xml");
    let doc = Invoice::new(
        g::FormatoTrasmissioneType::Fpr12,
        "00001",
        seller(),
        buyer_us(),
    )
    .body(
        InvoiceBody::new(
            g::TipoDocumentoType::Td01,
            "EUR",
            date(2026, 11, 4),
            "00012",
        )
        .causale("CONSULENZA AI SENSI DELL'ART. 7-TER DPR 633/72")
        .bollo_virtuale()
        .riferimento_normativo(g::NaturaType::N21, "ART. 7-TER DPR 633/72")
        .line(
            Line::new(
                1,
                "SERVIZIO DI CONSULENZA",
                amount("150.00"),
                amount("0.00"),
            )
            .natura(g::NaturaType::N21)
            .quantita(amount("1.00")),
        ),
    )
    .build()
    .expect("the invoice passes every check");
    assert_eq!(doc, fixture);
}

/// A foreign buyer that carries a domestic recipient code instead of its
/// `XXXXXXX` passes the schema, the element being a plain code, and
/// `validate` decides nothing on it, since the code is 00311, which needs
/// the IPA registry.
#[test]
fn td01_non_eu_us_client_missing_xxxxxxx() {
    let mut doc = fixture_round_trips("td01-7ter-non-eu-us.xml");
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "ABC1234".to_string();
    let written = doc.to_xml_vec().expect("the broken copy serialises");
    support::validate(&support::fpr12_schema(), &written)
        .expect("the schema accepts any 7 character code");
    assert!(
        validate(&doc).is_empty(),
        "the check is 00311, which needs the IPA registry"
    );
}

/// A mangled foreign recipient code of the wrong length for `FPR12` raises
/// 00427.
#[test]
fn td01_non_eu_us_client_wrong_length_code() {
    let mut doc = fixture_round_trips("td01-7ter-non-eu-us.xml");
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "XXXXXX".to_string();
    assert_has(&validate(&doc), "00427");
}

/// Fixture 2: services to an EU business client, the reverse charge resting
/// on the client, `N2.1` at a zero rate. The total is below the stamp duty
/// threshold, so no `DatiBollo` block is present.
#[test]
fn td01_services_to_an_eu_de_client() {
    let doc = fixture_round_trips("td01-7ter-eu-de.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    let header = &doc.content.fattura_elettronica_header.content;
    assert_eq!(
        header
            .dati_trasmissione
            .content
            .codice_destinatario
            .as_str(),
        "XXXXXXX"
    );
    let buyer = &header
        .cessionario_committente
        .content
        .dati_anagrafici
        .content;
    assert_eq!(
        buyer
            .id_fiscale_iva
            .as_ref()
            .expect("the buyer carries its VAT number")
            .content
            .id_paese,
        "DE"
    );
    assert_eq!(
        buyer
            .id_fiscale_iva
            .as_ref()
            .expect("the buyer carries its VAT number")
            .content
            .id_codice,
        "DE998877665"
    );
    let line = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dettaglio_linee[0]
        .content;
    assert_eq!(line.aliquota_iva, amount("0.00"));
    assert_eq!(line.natura, Some(g::NaturaType::N21));
    let generali = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content;
    assert_eq!(generali.dati_bollo, None);
    assert_eq!(generali.importo_totale_documento, Some(amount("50.00")));
}

/// The same invoice built with the `Invoice` builder equals the hand-checked
/// fixture.
#[test]
fn td01_eu_de_client_matches_the_builder() {
    let fixture = fixture_round_trips("td01-7ter-eu-de.xml");
    let doc = Invoice::new(
        g::FormatoTrasmissioneType::Fpr12,
        "00002",
        seller(),
        buyer_de(),
    )
    .body(
        InvoiceBody::new(
            g::TipoDocumentoType::Td01,
            "EUR",
            date(2026, 11, 5),
            "00013",
        )
        .causale("CONSULENZA INVERSE CHARGE AI SENSI DELL'ART. 7-TER DPR 633/72")
        .riferimento_normativo(g::NaturaType::N21, "ART. 7-TER DPR 633/72")
        .line(
            Line::new(1, "SERVIZIO DI CONSULENZA", amount("50.00"), amount("0.00"))
                .natura(g::NaturaType::N21)
                .quantita(amount("1.00")),
        ),
    )
    .build()
    .expect("the invoice passes every check");
    assert_eq!(doc, fixture);
}

/// A foreign buyer that carries a domestic recipient code instead of its
/// `XXXXXXX` passes the schema, and `validate` decides nothing on it, the
/// code being 00311, which needs the IPA registry.
#[test]
fn td01_eu_de_client_missing_xxxxxxx() {
    let mut doc = fixture_round_trips("td01-7ter-eu-de.xml");
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "ABC1234".to_string();
    let written = doc.to_xml_vec().expect("the broken copy serialises");
    support::validate(&support::fpr12_schema(), &written)
        .expect("the schema accepts any 7 character code");
    assert!(
        validate(&doc).is_empty(),
        "the check is 00311, which needs the IPA registry"
    );
}

/// A mangled foreign recipient code of the wrong length for `FPR12` raises
/// 00427.
#[test]
fn td01_eu_de_client_wrong_length_code() {
    let mut doc = fixture_round_trips("td01-7ter-eu-de.xml");
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "XXXXXX".to_string();
    assert_has(&validate(&doc), "00427");
}

/// Fixture 3: the `TD17` self-invoice for a service bought from an EU
/// supplier with Italian VAT due. The foreign supplier is the seller, the
/// Italian taxpayer is the buyer, the rate is 22 percent, the date is the
/// self-invoice date and the number is the one the taxpayer attributes to
/// the document, per supplier.
#[test]
fn td17_service_from_an_eu_supplier() {
    let doc = fixture_round_trips("td17-eu-supplier.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    let header = &doc.content.fattura_elettronica_header.content;
    let seller = &header
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva;
    assert_eq!(seller.content.id_paese, "DE");
    let buyer = &header
        .cessionario_committente
        .content
        .dati_anagrafici
        .content;
    assert_eq!(
        buyer
            .id_fiscale_iva
            .as_ref()
            .expect("the buyer carries its VAT number")
            .content
            .id_paese,
        "IT"
    );
    let generali = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content;
    assert_eq!(generali.tipo_documento, g::TipoDocumentoType::Td17);
    let line = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dettaglio_linee[0]
        .content;
    assert_eq!(line.aliquota_iva, amount("22.00"));
    let riep = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dati_riepilogo[0]
        .content;
    assert_eq!(riep.imponibile_importo, amount("100.00"));
    assert_eq!(riep.imposta, amount("22.00"));
    assert_eq!(generali.importo_totale_documento, Some(amount("122.00")));
}

/// An Italian seller on a `TD17` raises 00473.
#[test]
fn td17_eu_supplier_italian_seller() {
    let mut doc = fixture_round_trips("td17-eu-supplier.xml");
    doc.content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .content
        .id_paese = "IT".to_string();
    assert_has(&validate(&doc), "00473");
}

/// A `TD17` whose seller and buyer are identical raises 00471.
#[test]
fn td17_eu_supplier_identical_parties() {
    let mut doc = fixture_round_trips("td17-eu-supplier.xml");
    doc.content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva = doc
        .content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .clone()
        .expect("the buyer carries its VAT number");
    assert_has(&validate(&doc), "00471");
}

/// A `TD17` whose buyer sits outside Italy as well raises 00476.
#[test]
fn td17_eu_supplier_buyer_outside_italy() {
    let mut doc = fixture_round_trips("td17-eu-supplier.xml");
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .as_mut()
        .expect("the buyer carries its VAT number")
        .content
        .id_paese = "DE".to_string();
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .as_mut()
        .expect("the buyer carries its VAT number")
        .content
        .id_codice = "DE555444333".to_string();
    assert_has(&validate(&doc), "00476");
}

/// A `TD17` buyer without its `IdFiscaleIVA` raises 00475.
#[test]
fn td17_eu_supplier_buyer_without_iva() {
    let mut doc = fixture_round_trips("td17-eu-supplier.xml");
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva = None;
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content
        .codice_fiscale = Some("12345678901".to_string());
    assert_has(&validate(&doc), "00475");
}

/// Fixture 4: the `TD17` self-invoice for a service bought from a non-EU
/// supplier, 22 percent VAT, the date and the number following the same
/// rules as the EU case.
#[test]
fn td17_service_from_a_non_eu_supplier() {
    let doc = fixture_round_trips("td17-non-eu-supplier.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    let header = &doc.content.fattura_elettronica_header.content;
    let seller = &header
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva;
    assert_eq!(seller.content.id_paese, "US");
    let buyer = &header
        .cessionario_committente
        .content
        .dati_anagrafici
        .content;
    assert_eq!(
        buyer
            .id_fiscale_iva
            .as_ref()
            .expect("the buyer carries its VAT number")
            .content
            .id_paese,
        "IT"
    );
    let generali = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content;
    assert_eq!(generali.tipo_documento, g::TipoDocumentoType::Td17);
    let riep = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dati_riepilogo[0]
        .content;
    assert_eq!(riep.imponibile_importo, amount("200.00"));
    assert_eq!(riep.imposta, amount("44.00"));
    assert_eq!(generali.importo_totale_documento, Some(amount("244.00")));
}

/// An Italian seller on the non-EU `TD17` raises 00473.
#[test]
fn td17_non_eu_supplier_italian_seller() {
    let mut doc = fixture_round_trips("td17-non-eu-supplier.xml");
    doc.content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .content
        .id_paese = "IT".to_string();
    assert_has(&validate(&doc), "00473");
}

/// A non-EU `TD17` whose buyer sits outside Italy as well raises 00476.
#[test]
fn td17_non_eu_supplier_buyer_outside_italy() {
    let mut doc = fixture_round_trips("td17-non-eu-supplier.xml");
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .as_mut()
        .expect("the buyer carries its VAT number")
        .content
        .id_paese = "US".to_string();
    assert_has(&validate(&doc), "00476");
}

/// A non-EU `TD17` whose seller and buyer are identical raises 00471.
#[test]
fn td17_non_eu_supplier_identical_parties() {
    let mut doc = fixture_round_trips("td17-non-eu-supplier.xml");
    doc.content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva = doc
        .content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .clone()
        .expect("the buyer carries its VAT number");
    assert_has(&validate(&doc), "00471");
}
