//! The typed invoice construction: totals and rounding against
//! hand-computed figures, the stamp duty at the threshold, the discounts
//! and markups, the `FPA12` and `PEC` recipients and the domestic
//! invoice priced in a foreign currency.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

#[path = "support/mod.rs"]
mod support;

use fatturapa::generated::fpr12 as g;
use fatturapa::ids::{Natura, Provincia, RegimeFiscale, RiferimentoNormativo};
use fatturapa::invoice::{
    Invoice, InvoiceCurrency, Line, Recipient, Sconto, SelfInvoice, VatTreatment,
};
use fatturapa::party::{Address, Business, Buyer, Company, Party, Seller};
use fatturapa::{Amount, Document, FatturaElettronica};
use iso_country::Country;
use iso_currency::Currency;

fn amount(text: &str) -> Amount {
    text.parse().expect("a valid decimal")
}

fn seller() -> Seller {
    Seller::denomination(
        "ALPHA SRL".parse().expect("a valid denomination"),
        "02222222222".parse().expect("a valid partita IVA"),
        Address::italian(
            "VIA ROMA 1".parse().expect("a valid address"),
            "00100".parse().expect("a valid CAP"),
            "ROMA".parse().expect("a valid comune"),
            Provincia::Rm,
        ),
        RegimeFiscale::Rf01,
    )
}

fn buyer() -> Buyer {
    Buyer::denomination(
        "BETA SRL".parse().expect("a valid denomination"),
        "04444444444".parse().expect("a valid partita IVA"),
        Address::italian(
            "VIA MILANO 2".parse().expect("a valid address"),
            "20100".parse().expect("a valid CAP"),
            "MILANO".parse().expect("a valid comune"),
            Provincia::Mi,
        ),
    )
}

fn code() -> Recipient {
    Recipient::Codice("ABC1234".parse().expect("a valid code"))
}

fn taxed_line(description: &str, price: &str, quantity: &str, aliquota: &str) -> Line {
    Line::builder()
        .description(description)
        .quantity(quantity)
        .unit_price(price)
        .vat(VatTreatment::Taxed(aliquota.parse().expect("a valid rate")))
        .build()
        .expect("the line builds")
}

/// A domestic 22 percent invoice with several lines, a percentage discount
/// and an 8 decimal line, checked against hand-computed totals.
///
/// Line 1: 10.00 x 3 = 30.00 at 22.
/// Line 2: 5.55 x 3 = 16.65 at 22.
/// Line 3: (100.00 - 5%) x 3 = 285.00 at 22.
/// Line 4: 0.333333 x 3 = 0.999999 at 22.
/// `Imponibile` 22: 332.649999, rounded to 332.65, arrotondamento 0.000001.
/// Imposta 22: 332.65 x 22 / 100 = 73.183, rounded to 73.18.
/// Line 5: 1.11 x 2 = 2.22 at 10, imposta 0.222 rounded to 0.22.
/// `ImportoTotaleDocumento`: 332.65 + 73.18 + 2.22 + 0.22 = 408.27.
#[test]
fn domestic_invoice_totals_and_rounding() {
    let discount_line = Line::builder()
        .description("FORNITURA CON SCONTO")
        .quantity("3.00")
        .unit_price("100.00")
        .sconti(vec![
            Sconto::discount_percent("5.00").expect("a valid discount"),
        ])
        .vat(VatTreatment::Taxed("22.00".parse().expect("a valid rate")))
        .build()
        .expect("the line builds");
    let doc = Invoice::domestic(
        &seller(),
        &buyer(),
        code(),
        taxed_line("MATERIALE UFFICIO", "10.00", "3.00", "22.00"),
    )
    .lines(vec![
        taxed_line("CARTA", "5.55", "3.00", "22.00"),
        discount_line,
        taxed_line(
            "COMPONENTE A PREZZO UNITARIO FRAZIONARIO",
            "0.333333",
            "3.00",
            "22.00",
        ),
        taxed_line("SALDAZIONE IVA 10", "1.11", "2.00", "10.00"),
    ])
    .progressivo("00001")
    .date("2026-11-04")
    .numero("00013")
    .build()
    .expect("the invoice passes every check");

    let body = &doc.content.fattura_elettronica_body[0];
    let lines = &body.content.dati_beni_servizi.content.dettaglio_linee;
    assert_eq!(lines[0].content.prezzo_totale, amount("30.00"));
    assert_eq!(lines[1].content.prezzo_totale, amount("16.65"));
    assert_eq!(lines[2].content.prezzo_totale, amount("285.00"));
    assert_eq!(lines[3].content.prezzo_totale, amount("0.999999"));
    assert_eq!(lines[4].content.prezzo_totale, amount("2.22"));

    let rie = &body.content.dati_beni_servizi.content.dati_riepilogo;
    assert_eq!(rie[0].content.aliquota_iva, amount("22.00"));
    assert_eq!(rie[0].content.imponibile_importo, amount("332.65"));
    assert_eq!(rie[0].content.arrotondamento, Some(amount("0.000001")));
    assert_eq!(rie[0].content.imposta, amount("73.18"));
    assert_eq!(rie[1].content.aliquota_iva, amount("10.00"));
    assert_eq!(rie[1].content.imponibile_importo, amount("2.22"));
    assert_eq!(rie[1].content.arrotondamento, None);
    assert_eq!(rie[1].content.imposta, amount("0.22"));

    let generali = &body
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content;
    assert_eq!(generali.importo_totale_documento, Some(amount("408.27")));

    let bytes = doc.to_xml_vec().expect("the invoice serialises");
    support::validate(&support::fpr12_schema(), &bytes)
        .expect("the written invoice validates against the schema");
    let reread = FatturaElettronica::from_xml_slice(&bytes).expect("the invoice re-reads");
    assert_eq!(reread, doc);
}

/// The `PrezzoTotale` of a line applies a percentage discount on the
/// unit price, the 5 percent on 100.00 worth 5.00, and the same
/// percentage on every line of the riepilogo, the 332.65 imponibile
/// giving the 73.18 imposta checked in the totals test.
#[test]
fn a_percentage_discount_adjusts_the_unit_price() {
    let line = Line::builder()
        .description("FORNITURA CON SCONTO")
        .quantity("3.00")
        .unit_price("100.00")
        .sconti(vec![
            Sconto::discount_percent("5.00").expect("a valid discount"),
        ])
        .vat(VatTreatment::Taxed("22.00".parse().expect("a valid rate")))
        .build()
        .expect("the line builds");
    let doc = Invoice::domestic(&seller(), &buyer(), code(), line)
        .progressivo("00001")
        .date("2026-11-04")
        .numero("00014")
        .build()
        .expect("the invoice passes every check");

    let line = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dettaglio_linee[0]
        .content;
    // (100.00 - 5%) x 3 = 285.00.
    assert_eq!(line.prezzo_totale, amount("285.00"));
}

/// The line `PrezzoTotale` applies several `ScontoMaggiorazione` blocks in
/// row on the progressively adjusted unit price, the 10 percent discount
/// and the 10.00 euro markup canceling each other on the 100.00 unit
/// price.
#[test]
fn line_total_applies_several_sconto_maggiorazione_blocks() {
    let line = Line::builder()
        .description("FORNITURA CON SCONTO E MAGGIORAZIONE")
        .quantity("1.00")
        .unit_price("100.00")
        .sconti(vec![
            Sconto::discount_percent("10.00").expect("a valid discount"),
            Sconto::markup_amount("10.00").expect("a valid markup"),
        ])
        .vat(VatTreatment::Taxed("22.00".parse().expect("a valid rate")))
        .build()
        .expect("the line builds");
    let doc = Invoice::domestic(&seller(), &buyer(), code(), line)
        .progressivo("00001")
        .date("2026-11-04")
        .numero("00022")
        .build()
        .expect("the invoice passes every check");

    let line = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dettaglio_linee[0]
        .content;
    assert_eq!(line.prezzo_totale, amount("100.00"));
}

/// The stamp duty applies strictly above 77.47, so a total of exactly
/// 77.47 declares the virtual stamp duty without the 2.00 amount and
/// 77.48 writes it.
#[test]
fn stamp_duty_applies_strictly_above_the_threshold() {
    for price in ["77.47", "77.48"] {
        let doc = Invoice::domestic(
            &seller(),
            &buyer(),
            code(),
            Line::builder()
                .description("FORNITURA ESCLUSA")
                .quantity("1.00")
                .unit_price(price)
                .vat(VatTreatment::NotTaxed {
                    natura: Natura::N2_1,
                    riferimento_normativo: "ART. 7-TER DPR 633/72"
                        .parse::<RiferimentoNormativo>()
                        .expect("a valid reference"),
                })
                .build()
                .expect("the line builds"),
        )
        .bollo(true)
        .progressivo("00001")
        .date("2026-11-04")
        .numero("00021")
        .build()
        .expect("the invoice passes every check");

        let generali = &doc.content.fattura_elettronica_body[0]
            .content
            .dati_generali
            .content
            .dati_generali_documento
            .content;
        assert_eq!(generali.importo_totale_documento, Some(amount(price)));
        let bollo = generali
            .dati_bollo
            .as_ref()
            .expect("the stamp duty is present");
        assert_eq!(bollo.content.bollo_virtuale, g::BolloVirtualeType::Si);
        assert_eq!(
            bollo.content.importo_bollo,
            (price == "77.48").then_some(amount("2.00"))
        );
    }
}

/// An `FPA12` invoice from a six character recipient code assembles and
/// validates, the format following the length of the code.
#[test]
fn fpa12_invoice_from_a_six_character_code() {
    let doc = Invoice::domestic(
        &seller(),
        &buyer(),
        Recipient::Codice("AAAAAA".parse().expect("a valid code")),
        taxed_line("FORNITURA", "100.00", "1.00", "22.00"),
    )
    .progressivo("00001")
    .date("2026-11-04")
    .numero("00019")
    .build()
    .expect("the invoice passes every check");
    assert_eq!(doc.versione, g::FormatoTrasmissioneType::Fpa12);

    let bytes = doc.to_xml_vec().expect("the invoice serialises");
    support::validate(&support::fpr12_schema(), &bytes)
        .expect("the written invoice validates against the schema");
}

/// A `PEC` recipient writes the `CodiceDestinatario` `0000000` and the
/// `PECDestinatario` of the buyer in the `FPR12` format.
#[test]
fn the_pec_recipient_writes_the_generic_code_and_the_pec() {
    let doc = Invoice::domestic(
        &seller(),
        &buyer(),
        Recipient::Pec("beta@pec.it".parse().expect("a valid pec")),
        taxed_line("FORNITURA", "100.00", "1.00", "22.00"),
    )
    .progressivo("00001")
    .date("2026-11-04")
    .numero("00025")
    .build()
    .expect("the invoice passes every check");

    let transmission = &doc
        .content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content;
    assert_eq!(transmission.codice_destinatario.as_str(), "0000000");
    assert_eq!(
        transmission.pec_destinatario.as_deref(),
        Some("beta@pec.it")
    );
    assert_eq!(doc.versione, g::FormatoTrasmissioneType::Fpr12);
}

/// A `DatiRiepilogo` per distinct rate and nature keeps its insertion
/// order, the second 22 percent line joining the first riepilogo.
#[test]
fn riepilogo_order_follows_the_lines() {
    let doc = Invoice::domestic(
        &seller(),
        &buyer(),
        code(),
        taxed_line("COMPONENTE 22", "10.00", "1.00", "22.00"),
    )
    .lines(vec![
        taxed_line("COMPONENTE 10", "20.00", "1.00", "10.00"),
        taxed_line("COMPONENTE 22 BIS", "30.00", "1.00", "22.00"),
    ])
    .progressivo("00001")
    .date("2026-11-04")
    .numero("00020")
    .build()
    .expect("the invoice passes every check");

    let rie = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dati_riepilogo;
    assert_eq!(rie.len(), 2);
    assert_eq!(rie[0].content.aliquota_iva, amount("22.00"));
    assert_eq!(rie[0].content.imponibile_importo, amount("40.00"));
    assert_eq!(rie[0].content.imposta, amount("8.80"));
    assert_eq!(rie[1].content.aliquota_iva, amount("10.00"));
    assert_eq!(rie[1].content.imponibile_importo, amount("20.00"));
    assert_eq!(rie[1].content.imposta, amount("2.00"));

    let generali = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content;
    assert_eq!(generali.importo_totale_documento, Some(amount("70.80")));
}

/// A domestic invoice priced in US dollars keeps the `Divisa` `EUR`, the
/// 150.00 dollar line at the 1.08 rate worth 138.89 euros, the original
/// amount and the rate carried only as management data of the line.
#[test]
fn domestic_invoice_priced_in_usd() {
    let doc = Invoice::domestic(
        &seller(),
        &buyer(),
        code(),
        taxed_line("SERVIZIO IN DOLLARI", "150.00", "1.00", "22.00"),
    )
    .progressivo("00001")
    .date("2026-11-04")
    .numero("00026")
    .currency(InvoiceCurrency::Foreign {
        currency: Currency::USD,
        rate: "1.08".parse().expect("a valid rate"),
        rate_date: "2026-11-04".parse().expect("a valid date"),
    })
    .build()
    .expect("the invoice passes every check");

    let body = &doc.content.fattura_elettronica_body[0].content;
    let generali = &body.dati_generali.content.dati_generali_documento.content;
    assert_eq!(generali.divisa, "EUR");
    let line = &body.dati_beni_servizi.content.dettaglio_linee[0].content;
    // 150.00 USD at 1.08 USD per euro, the euro value 138.89.
    assert_eq!(line.prezzo_unitario, amount("138.89"));
    assert_eq!(line.prezzo_totale, amount("138.89"));
    let gestione = &line.altri_dati_gestionali;
    assert_eq!(gestione.len(), 2);
    assert_eq!(gestione[0].content.tipo_dato, "USD AMT");
    assert_eq!(
        gestione[0].content.riferimento_numero,
        Some(amount("150.00"))
    );
    assert_eq!(gestione[1].content.tipo_dato, "USD RATE");
    assert_eq!(
        gestione[1].content.riferimento_numero,
        Some(amount("1.0800"))
    );
    let riep = &body.dati_beni_servizi.content.dati_riepilogo[0].content;
    assert_eq!(riep.imponibile_importo, amount("138.89"));
    assert_eq!(riep.imposta, amount("30.56"));
    // 138.89 + 30.56 = 169.45.
    assert_eq!(generali.importo_totale_documento, Some(amount("169.45")));

    let bytes = doc.to_xml_vec().expect("the invoice serialises");
    support::validate(&support::fpr12_schema(), &bytes)
        .expect("the written invoice validates against the schema");
}

/// The `Nome`, the `Cognome`, the `IdFiscaleIVA` and the `CodiceFiscale`
/// element of a sole-trader seller and a sole-trader buyer, and the
/// conversion of each role into the `Party` it carries.
#[test]
fn sole_trader_parties_fill_the_header() {
    let seller = Seller::sole_trader(
        "MARIO".parse().expect("a valid name"),
        "ROSSI".parse().expect("a valid name"),
        "02222222222".parse().expect("a valid partita IVA"),
        "MRALNE80E05H501C".parse().expect("a valid codice fiscale"),
        Address::italian(
            "VIA ROMA 1".parse().expect("a valid address"),
            "00100".parse().expect("a valid CAP"),
            "ROMA".parse().expect("a valid comune"),
            Provincia::Rm,
        ),
        RegimeFiscale::Rf01,
    );
    let buyer = Buyer::sole_trader(
        "PAOLO".parse().expect("a valid name"),
        "VERDI".parse().expect("a valid name"),
        "04444444444".parse().expect("a valid partita IVA"),
        "RSSMRA80A01H501U".parse().expect("a valid codice fiscale"),
        Address::italian(
            "VIA MILANO 2".parse().expect("a valid address"),
            "20100".parse().expect("a valid CAP"),
            "MILANO".parse().expect("a valid comune"),
            Provincia::Mi,
        ),
    );
    let doc = Invoice::domestic(
        &seller,
        &buyer,
        code(),
        taxed_line("MATERIALE UFFICIO", "10.00", "1.00", "22.00"),
    )
    .progressivo("00001")
    .date("2026-11-04")
    .numero("00014")
    .build()
    .expect("the invoice passes every check");

    let header = &doc.content.fattura_elettronica_header.content;
    let cedente = &header.cedente_prestatore.content.dati_anagrafici.content;
    assert_eq!(cedente.id_fiscale_iva.content.id_paese, "IT");
    assert_eq!(cedente.id_fiscale_iva.content.id_codice, "02222222222");
    assert_eq!(cedente.codice_fiscale.as_deref(), Some("MRALNE80E05H501C"));
    let (nome, cognome) = match &cedente.anagrafica.content.content_60.content {
        g::AnagraficaContent60TypeContent::Content62(g::AnagraficaContent62Type {
            content: g::AnagraficaContent62TypeContent { nome, cognome },
        }) => (nome.clone(), cognome.clone()),
        g::AnagraficaContent60TypeContent::Content61(_) => {
            panic!("the sole trader carries the Nome and the Cognome")
        }
    };
    assert_eq!(nome, "MARIO");
    assert_eq!(cognome, "ROSSI");

    let cessionario = &header
        .cessionario_committente
        .content
        .dati_anagrafici
        .content;
    assert_eq!(
        cessionario
            .id_fiscale_iva
            .as_ref()
            .expect("the buyer carries its IdFiscaleIVA")
            .content
            .id_codice,
        "04444444444"
    );
    assert_eq!(
        cessionario.codice_fiscale.as_deref(),
        Some("RSSMRA80A01H501U")
    );
    assert!(
        matches!(
            cessionario.anagrafica.content.content_60.content,
            g::AnagraficaContent60TypeContent::Content62(_)
        ),
        "the buyer carries the Nome and the Cognome"
    );

    let seller_party: Party = seller.clone().into();
    assert_eq!(
        seller_party.identity().id_fiscale_iva(),
        Some(("IT".to_string(), "02222222222".to_string()))
    );
    assert_eq!(seller_party.address().cap(), "00100");
    assert_eq!(seller_party.address().comune(), "ROMA");
    assert_eq!(seller_party.address().provincia_code(), Some("RM"));
    assert_eq!(seller_party.address().country_code(), "IT");
    assert_eq!(seller_party.name(), seller.party().name());

    let buyer_party: Party = buyer.clone().into();
    assert_eq!(
        buyer_party.identity().id_fiscale_iva(),
        Some(("IT".to_string(), "04444444444".to_string()))
    );
    assert_eq!(buyer_party.address().cap(), "20100");
    assert_eq!(buyer_party.name(), buyer.party().name());

    let bytes = doc.to_xml_vec().expect("the invoice serialises");
    support::validate(&support::fpr12_schema(), &bytes)
        .expect("the written invoice validates against the schema");
}

/// A `TD17` self-invoice whose Italian buyer is a sole trader with its
/// `partita IVA`: the buyer transmits the document, the foreign supplier
/// carries the `RF01` regime, and the two roles convert into the
/// `Party` they carry.
#[test]
fn td17_self_invoice_with_a_sole_trader_buyer() {
    let company = Company::sole_trader(
        "PAOLO".parse().expect("a valid name"),
        "VERDI".parse().expect("a valid name"),
        "04444444444".parse().expect("a valid partita IVA"),
        "RSSMRA80A01H501U".parse().expect("a valid codice fiscale"),
        Address::italian(
            "VIA MILANO 2".parse().expect("a valid address"),
            "20100".parse().expect("a valid CAP"),
            "MILANO".parse().expect("a valid comune"),
            Provincia::Mi,
        ),
    );
    let supplier = Business::foreign(
        Country::US,
        "ACME INC".parse().expect("a valid denomination"),
        "US123456789".parse().expect("a valid identifier"),
        Address::foreign(
            "FIFTH AVENUE 1".parse().expect("a valid address"),
            Some("10001".parse().expect("a valid postal code")),
            "NEW YORK".parse().expect("a valid comune"),
            Country::US,
        )
        .expect("a valid foreign seat"),
    );
    let doc = SelfInvoice::td17(
        &supplier,
        &company,
        code(),
        taxed_line("SERVIZIO DI CONSULENZA", "100.00", "1.00", "22.00"),
    )
    .progressivo("00001")
    .date("2026-11-04")
    .numero("00015")
    .original_invoice("ORIG-001", "2026-11-01")
    .build()
    .expect("the self-invoice passes every check");

    let header = &doc.content.fattura_elettronica_header.content;
    let trasmissione = &header.dati_trasmissione.content;
    assert_eq!(trasmissione.id_trasmittente.content.id_paese, "IT");
    assert_eq!(
        trasmissione.id_trasmittente.content.id_codice,
        "04444444444"
    );
    let cedente = &header.cedente_prestatore.content.dati_anagrafici.content;
    assert_eq!(cedente.regime_fiscale, g::RegimeFiscaleType::Rf01);
    let cessionario = &header
        .cessionario_committente
        .content
        .dati_anagrafici
        .content;
    assert_eq!(
        cessionario
            .id_fiscale_iva
            .as_ref()
            .expect("the buyer carries its IdFiscaleIVA")
            .content
            .id_codice,
        "04444444444"
    );
    assert_eq!(
        cessionario.codice_fiscale.as_deref(),
        Some("RSSMRA80A01H501U")
    );
    let body = &doc.content.fattura_elettronica_body[0].content;
    assert_eq!(
        body.dati_generali
            .content
            .dati_generali_documento
            .content
            .tipo_documento,
        g::TipoDocumentoType::Td17
    );
    let collegate = &body.dati_generali.content.dati_fatture_collegate;
    assert_eq!(collegate.len(), 1);
    assert_eq!(collegate[0].content.id_documento, "ORIG-001");

    let company_party: Party = company.clone().into();
    assert_eq!(
        company_party.identity().id_fiscale_iva(),
        Some(("IT".to_string(), "04444444444".to_string()))
    );
    assert_eq!(company_party.address().cap(), "20100");
    assert_eq!(company_party.name(), company.party().name());

    let supplier_party: Party = supplier.clone().into();
    assert_eq!(
        supplier_party.identity().id_fiscale_iva(),
        Some(("US".to_string(), "US123456789".to_string()))
    );
    assert_eq!(supplier_party.address().cap(), "00000");
    assert_eq!(supplier_party.address().indirizzo(), "FIFTH AVENUE 1 10001");
    assert_eq!(supplier_party.address().country_code(), "US");
    assert_eq!(supplier_party.name(), supplier.party().name());

    let bytes = doc.to_xml_vec().expect("the self-invoice serialises");
    support::validate(&support::fpr12_schema(), &bytes)
        .expect("the written self-invoice validates against the schema");
}
