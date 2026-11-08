//! One fixture per missing FatturaPA document type, anonymized from the
//! public sources named in `tests/fixtures/README.md` or written by hand
//! against the Guida alla compilazione 1.10. Each fixture is validated
//! against the official schema, read back, round-tripped and checked with
//! `validate` or `validate_simplified`, each carries the type-specific
//! assertion a plausible bug would break, and each broken copy raises the
//! Elenco Controlli code it should.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

#[path = "support/mod.rs"]
mod support;

use support::{fixture_round_trips as fpr12_round_trips, fsm10_round_trips};

use fatturapa::generated::fpr12 as g;
use fatturapa::generated::fsm10 as s;
use fatturapa::validate::{validate, validate_simplified};
use fatturapa::{Amount, FatturaElettronica};

fn amount(text: &str) -> Amount {
    text.parse().expect("a valid decimal")
}

/// The seller `IdFiscaleIVA` of an ordinary invoice.
const fn seller_vat(doc: &FatturaElettronica) -> &g::IdFiscaleType {
    &doc.content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
}

/// The buyer `IdFiscaleIVA` of an ordinary invoice, present where 00475
/// or the scenario requires it.
const fn buyer_vat(doc: &FatturaElettronica) -> &g::IdFiscaleType {
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .as_ref()
        .expect("the buyer carries its VAT number")
}

/// The `DatiGeneraliDocumento` of the single body of an ordinary invoice.
fn generali(doc: &FatturaElettronica) -> &g::DatiGeneraliDocumentoTypeContent {
    &doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
}

/// The `DatiFattureCollegate` of the single body of an ordinary invoice.
fn collegate(doc: &FatturaElettronica) -> &[g::DatiDocumentiCorrelatiType] {
    &doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_fatture_collegate
}

/// Fixture: the `TD02` down payment, which precedes the balance invoice
/// and carries no `DatiFattureCollegate` link.
#[test]
fn td02_advance_is_a_plain_advance() {
    let doc = fpr12_round_trips("td02-advance.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td02);
    assert!(
        collegate(&doc).is_empty(),
        "the down payment links no document"
    );
    assert_ne!(
        seller_vat(&doc).content.id_codice,
        buyer_vat(&doc).content.id_codice,
        "the down payment names two different parties"
    );
}

/// Fixture: the `TD01` balance invoice that deducts the down payment,
/// with the `DatiFattureCollegate` link the deduction requires and a
/// negative line for the deducted amount.
#[test]
fn td01_balance_references_the_advance() {
    let doc = fpr12_round_trips("td01-advance-balance.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    let link = collegate(&doc)
        .first()
        .expect("the balance links the down payment");
    assert_eq!(link.content.id_documento, "00001");
    assert_eq!(
        link.content.data,
        Some("2025-09-10".parse().expect("a valid date"))
    );
    let lines = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dettaglio_linee;
    assert_eq!(lines[1].content.prezzo_totale, amount("-300.00"));
    assert_eq!(
        generali(&doc).importo_totale_documento,
        Some(amount("854.00"))
    );
}

/// Fixture: the `TD03` down payment for professional fees, issued by a
/// sole trader whose block carries the `CodiceFiscale`.
#[test]
fn td03_advance_parcella_from_a_sole_trader() {
    let doc = fpr12_round_trips("td03-advance-parcella.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td03);
    let seller = &doc
        .content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content;
    assert_eq!(seller.codice_fiscale.as_deref(), Some("MRALNE80E05H501C"));
}

/// Fixture: the `TD05` debit note that completes the missing
/// `IdDocumento` of the source, linking the invoice it amends.
#[test]
fn td05_debit_note_links_its_invoice() {
    let doc = fpr12_round_trips("td05-debit-note.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td05);
    let link = collegate(&doc)
        .first()
        .expect("the debit note links its invoice");
    assert_eq!(link.content.id_documento, "FT/2025/00001");
    assert_eq!(
        link.content.data,
        Some("2025-03-24".parse().expect("a valid date"))
    );
}

/// Fixture: the simplified `TD09` debit note, the `FSM10` form of the
/// debit note, with the `DatiFatturaRettificata` link to the invoice it
/// amends.
#[test]
fn td09_simplified_debit_note_links_its_invoice() {
    let doc = fsm10_round_trips("td09-simplified-debit-note.xml");
    assert!(
        validate_simplified(&doc).is_empty(),
        "the fixture passes every simplified check"
    );
    let generali = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content;
    assert_eq!(generali.tipo_documento, s::TipoDocumentoType::Td09);
    let rettificata = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_fattura_rettificata
        .as_ref()
        .expect("the debit note links the invoice it amends")
        .content;
    assert_eq!(rettificata.numero_fr, "0000000000001/2025");
    assert_eq!(
        rettificata.data_fr,
        "2025-11-10".parse().expect("a valid date")
    );
    let beni = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi[0]
        .content;
    assert_eq!(beni.importo, amount("50.00"));
    assert_eq!(beni.dati_iva.content.imposta, Some(amount("11.00")));
}

/// Fixture: the `TD25` deferred invoice of the triangular transaction,
/// issued by the seller like an ordinary invoice, with no linked
/// document.
#[test]
fn td25_triangular_deferred_invoice() {
    let doc = fpr12_round_trips("td25-triangolazione.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td25);
    assert_ne!(
        seller_vat(&doc).content.id_codice,
        buyer_vat(&doc).content.id_codice
    );
    assert_eq!(collegate(&doc), []);
}

/// Fixture: the `TD26` sale of an amortizable good that does not count
/// toward the turnover, issued by the seller to the buyer.
#[test]
fn td26_amortizable_good() {
    let doc = fpr12_round_trips("td26-amortizable.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td26);
    assert_ne!(
        seller_vat(&doc).content.id_codice,
        buyer_vat(&doc).content.id_codice
    );
}
