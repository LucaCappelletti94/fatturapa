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

use support::{assert_has, fixture_round_trips as fpr12_round_trips, fsm10_round_trips};

use fatturapa::generated::fpr12 as g;
use fatturapa::generated::fsm10 as s;
use fatturapa::invoice::{Line, Recipient, VatTreatment};
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

/// A single taxed line, with the rate of `rate`.
fn taxed_line(description: &str, price: &str, rate: &str) -> Line {
    Line::builder()
        .description(description)
        .quantity("1.00")
        .unit_price(price)
        .vat(VatTreatment::Taxed(rate.parse().expect("a valid rate")))
        .build()
        .expect("the line builds")
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

/// Fixture: the `TD18` integration of an intracommunity goods purchase,
/// the French seller with the `RF18` regime of the source, the buyer
/// transmits the document.
#[test]
fn td18_intracommunity_goods() {
    let doc = fpr12_round_trips("td18-intracommunity-goods.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td18);
    assert_eq!(seller_vat(&doc).content.id_paese, "FR");
    assert_eq!(buyer_vat(&doc).content.id_paese, "IT");
    assert_eq!(
        doc.content
            .fattura_elettronica_header
            .content
            .dati_trasmissione
            .content
            .id_trasmittente
            .content
            .id_codice,
        "04444444444"
    );
    let riep = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dati_riepilogo[0]
        .content;
    assert_eq!(riep.esigibilita_iva, Some(g::EsigibilitaIvaType::I));
    assert_eq!(
        collegate(&doc)
            .first()
            .expect("the integration links its purchase")
            .content
            .id_documento,
        "447/2025"
    );
}

/// A `TD18` whose seller sits in Italy raises 00473.
#[test]
fn td18_goods_italian_seller() {
    let mut doc = fpr12_round_trips("td18-intracommunity-goods.xml");
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

/// A `TD18` whose buyer sits outside Italy as well raises 00476.
#[test]
fn td18_goods_buyer_outside_italy() {
    let mut doc = fpr12_round_trips("td18-intracommunity-goods.xml");
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
        .id_paese = "FR".to_string();
    assert_has(&validate(&doc), "00476");
}

/// Fixture: the `TD18` credit note of the foreign seller, negative
/// amounts, linking the integration it reverses.
#[test]
fn td18_credit_note() {
    let doc = fpr12_round_trips("td18-credit-note.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td18);
    assert_eq!(seller_vat(&doc).content.id_paese, "FR");
    assert_eq!(
        generali(&doc).importo_totale_documento,
        Some(amount("-832.42"))
    );
    let link = collegate(&doc)
        .first()
        .expect("the credit note links the integration it reverses");
    assert_eq!(link.content.id_documento, "00001");
    assert_eq!(
        link.content.data,
        Some("2025-10-06".parse().expect("a valid date"))
    );
}

/// A `TD18` credit note whose seller sits in Italy raises 00473.
#[test]
fn td18_credit_italian_seller() {
    let mut doc = fpr12_round_trips("td18-credit-note.xml");
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

/// Fixture: the `TD19` self-invoice for goods already present in Italy,
/// the French seller in the `CedentePrestatore` block, the buyer
/// transmits the document.
#[test]
fn td19_goods_in_italy() {
    let doc = fpr12_round_trips("td19-goods-in-italy.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td19);
    assert_eq!(seller_vat(&doc).content.id_paese, "FR");
    assert_eq!(buyer_vat(&doc).content.id_paese, "IT");
    let riep = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dati_riepilogo;
    assert_eq!(riep.len(), 2);
    assert_eq!(riep[0].content.imponibile_importo, amount("800.40"));
    assert_eq!(riep[1].content.aliquota_iva, amount("4.00"));
    assert_eq!(
        generali(&doc).importo_totale_documento,
        Some(amount("1392.49"))
    );
}

/// The same self-invoice built with the `SelfInvoice::td19` constructor
/// equals the hand-checked fixture.
#[test]
fn td19_matches_the_constructor() {
    let fixture = fpr12_round_trips("td19-goods-in-italy.xml");
    let doc = fatturapa::invoice::SelfInvoice::td19(
        &support::business_fr(),
        &support::buyer_company(),
        Recipient::Codice("ABC1234".parse().expect("a valid code")),
        taxed_line("BENI GIA PRESENTI IN ITALIA", "800.40", "22.00"),
    )
    .progressivo("00001")
    .date("2025-11-05")
    .numero("001")
    .lines(vec![taxed_line(
        "BENI A ALIQUOTA RIDOTTA GIA PRESENTI IN ITALIA",
        "400.00",
        "4.00",
    )])
    .causali(vec![
        "AUTOFATTURA PER ACQUISTO DI BENI GIA PRESENTI IN ITALIA, AI SENSI DELL'ART. 17, COMMA 2, DPR 633/72"
            .parse()
            .expect("a valid causale"),
    ])
    .original_invoice("778/2025", "2025-10-28")
    .build()
    .expect("the self-invoice passes every check");
    assert_eq!(doc, fixture);
}

/// A `TD19` whose seller sits in Italy raises 00473.
#[test]
fn td19_italian_seller() {
    let mut doc = fpr12_round_trips("td19-goods-in-italy.xml");
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

/// Fixture: the `TD20` regularization of a payment split, the
/// `SoggettoEmittente` `CC` of the buyer and the `N6.4` nature on both
/// the line and the riepilogo.
#[test]
fn td20_regularization() {
    let doc = fpr12_round_trips("td20-regularization.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td20);
    assert_eq!(
        doc.content
            .fattura_elettronica_header
            .content
            .soggetto_emittente,
        Some(g::SoggettoEmittenteType::Cc)
    );
    let body = &doc.content.fattura_elettronica_body[0].content;
    assert_eq!(
        body.dati_beni_servizi.content.dettaglio_linee[0]
            .content
            .natura,
        Some(g::NaturaType::N64)
    );
    assert_eq!(
        body.dati_beni_servizi.content.dati_riepilogo[0]
            .content
            .natura,
        Some(g::NaturaType::N64)
    );
}

/// Fixture: the `TD21` self-invoice of the splafonamento, which 00472
/// requires to name the same party twice.
#[test]
fn td21_splafonamento() {
    let doc = fpr12_round_trips("td21-splafonamento.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td21);
    assert_eq!(
        seller_vat(&doc).content.id_codice,
        buyer_vat(&doc).content.id_codice
    );
    let lines = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dettaglio_linee;
    let sconto = &lines[1].content.sconto_maggiorazione[0].content;
    assert_eq!(sconto.tipo, g::TipoScontoMaggiorazioneType::Sc);
    assert_eq!(sconto.percentuale, Some(amount("10.00")));
    assert_eq!(lines[1].content.prezzo_totale, amount("450.00"));
}

/// A `TD21` whose seller and buyer differ raises 00472.
#[test]
fn td21_different_parties() {
    let mut doc = fpr12_round_trips("td21-splafonamento.xml");
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
        .id_codice = "04444444444".to_string();
    assert_has(&validate(&doc), "00472");
}

/// Fixture: the `TD22` extraction from an `IVA` deposit, case `A1`, the
/// German seller of the intracommunity purchase and the `NellAnno`
/// management data of the extraction in the same period.
#[test]
fn td22_deposit_extraction() {
    let doc = fpr12_round_trips("td22-deposito-extraction.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td22);
    assert_eq!(seller_vat(&doc).content.id_paese, "DE");
    let line = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dettaglio_linee[0]
        .content;
    assert_eq!(line.altri_dati_gestionali[0].content.tipo_dato, "NellAnno");
    assert_eq!(
        collegate(&doc)
            .first()
            .expect("the extraction links its purchase")
            .content
            .id_documento,
        "117/2025"
    );
}

/// Fixture: the `TD23` extraction from an `IVA` deposit with the `IVA`
/// paid, the Italian seller of the deposit operation.
#[test]
fn td23_deposit_payment() {
    let doc = fpr12_round_trips("td23-deposito-payment.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td23);
    assert_eq!(seller_vat(&doc).content.id_paese, "IT");
    assert_eq!(
        collegate(&doc)
            .first()
            .expect("the extraction links its purchase")
            .content
            .id_documento,
        "034/2025"
    );
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

/// Fixture: the `TD27` invoice of the autoconsumption, which 00472
/// requires to name the same party twice.
#[test]
fn td27_autoconsumo() {
    let doc = fpr12_round_trips("td27-autoconsumo.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td27);
    assert_eq!(
        seller_vat(&doc).content.id_codice,
        buyer_vat(&doc).content.id_codice
    );
}

/// A `TD27` whose seller and buyer differ raises 00472.
#[test]
fn td27_different_parties() {
    let mut doc = fpr12_round_trips("td27-autoconsumo.xml");
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
        .id_codice = "04444444444".to_string();
    assert_has(&validate(&doc), "00472");
}

/// Fixture: the `TD28` communication of a paper invoice with `IVA` from
/// a San Marino operator, the San Marino seller with the `RF18` regime
/// of the source.
#[test]
fn td28_san_marino_paper_invoice() {
    let doc = fpr12_round_trips("td28-san-marino-paper.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    assert_eq!(generali(&doc).tipo_documento, g::TipoDocumentoType::Td28);
    assert_eq!(seller_vat(&doc).content.id_paese, "SM");
    assert_eq!(buyer_vat(&doc).content.id_paese, "IT");
    assert_eq!(
        collegate(&doc)
            .first()
            .expect("the communication links the paper invoice")
            .content
            .id_documento,
        "FT-77/2025"
    );
}

/// A `TD28` whose seller sits in Italy raises 00473.
#[test]
fn td28_italian_seller() {
    let mut doc = fpr12_round_trips("td28-san-marino-paper.xml");
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
