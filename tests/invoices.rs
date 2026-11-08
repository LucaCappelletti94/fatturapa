//! The cross-border invoice fixtures, hand-written against the
//! specification. Each fixture is validated against the official schema,
//! read back, round-tripped and checked with `validate`, each scenario
//! constructor rebuilds its fixture exactly, and each broken copy raises
//! the Elenco Controlli code it should.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

#[path = "support/mod.rs"]
mod support;

use support::{assert_has, fixture_round_trips};

use fatturapa::generated::fpr12 as g;
use fatturapa::ids::{Natura, Provincia, RegimeFiscale};
use fatturapa::invoice::{Invoice, InvoiceCurrency, Line, Recipient, SevenTerLine, VatTreatment};
use fatturapa::party::{Address, Business, Company, Seller};
use fatturapa::validate::validate;
use fatturapa::{Amount, Document, FatturaElettronica};
use iso_currency::Currency;

fn amount(text: &str) -> Amount {
    text.parse().expect("a valid decimal")
}

fn seven_ter_line(description: &str, price: &str) -> SevenTerLine {
    SevenTerLine::builder()
        .description(description)
        .quantity("1.00")
        .unit_price(price)
        .build()
        .expect("the line builds")
}

fn taxed_line(description: &str, price: &str) -> Line {
    Line::builder()
        .description(description)
        .quantity("1.00")
        .unit_price(price)
        .vat(VatTreatment::Taxed("22.00".parse().expect("a valid rate")))
        .build()
        .expect("the line builds")
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
    let seller_seat = &doc
        .content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .sede
        .content;
    assert_eq!(seller_seat.provincia.as_deref(), Some("RM"));
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

/// The same invoice built with the `cross_border_services` constructor
/// equals the hand-checked fixture, pinning the constructor to it.
#[test]
fn td01_non_eu_us_client_matches_the_constructor() {
    let fixture = fixture_round_trips("td01-7ter-non-eu-us.xml");
    let doc = Invoice::cross_border_services(
        &support::seller(),
        &support::business_us(),
        seven_ter_line("SERVIZIO DI CONSULENZA", "150.00"),
    )
    .progressivo("00001")
    .date("2026-11-04")
    .numero("00012")
    .causali(vec![
        "CONSULENZA AI SENSI DELL'ART. 7-TER DPR 633/72"
            .parse()
            .expect("a valid causale"),
    ])
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
/// on the client, `N2.1` at a zero rate. The total of 50.00 is at most
/// 77.47 EUR, so the document writes no `DatiBollo` block.
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
    let buyer_seat = &doc
        .content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .sede
        .content;
    assert_eq!(buyer_seat.indirizzo, "BEISPIELSTRASSE 1 10115");
    assert_eq!(buyer_seat.cap, "00000");
    assert_eq!(buyer_seat.provincia, None);
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
    assert!(
        generali.dati_bollo.is_none(),
        "the total at most 77.47 writes no stamp duty"
    );
    assert_eq!(generali.importo_totale_documento, Some(amount("50.00")));
}

/// The same invoice built with the `cross_border_services` constructor
/// equals the hand-checked fixture.
#[test]
fn td01_eu_de_client_matches_the_constructor() {
    let fixture = fixture_round_trips("td01-7ter-eu-de.xml");
    let doc = Invoice::cross_border_services(
        &support::seller(),
        &support::business_de(),
        seven_ter_line("SERVIZIO DI CONSULENZA", "50.00"),
    )
    .progressivo("00002")
    .date("2026-11-05")
    .numero("00013")
    .causali(vec![
        "CONSULENZA INVERSE CHARGE AI SENSI DELL'ART. 7-TER DPR 633/72"
            .parse()
            .expect("a valid causale"),
    ])
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

/// Fixture 3: the art. 7-ter services invoice to a non-EU business client,
/// priced in US dollars. The `Divisa` stays `EUR` and every amount the
/// `SDI` reads in euros, the art. 13 convention, the 150.00 dollar line at
/// the 1.08 rate worth 138.89 euros, the original amount and the rate
/// carried only as management data of the line.
#[test]
fn td01_services_to_a_non_eu_us_client_priced_in_usd() {
    let doc = fixture_round_trips("td01-7ter-non-eu-usd.xml");
    assert!(validate(&doc).is_empty(), "the fixture passes every check");
    let generali = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content;
    assert_eq!(generali.divisa, "EUR");
    assert_eq!(generali.importo_totale_documento, Some(amount("138.89")));
    let line = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dettaglio_linee[0]
        .content;
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
    assert_eq!(
        gestione[1].content.riferimento_data,
        Some("2026-11-04".parse().expect("a valid date"))
    );
    let riep = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dati_riepilogo[0]
        .content;
    assert_eq!(riep.imponibile_importo, amount("138.89"));
    assert_eq!(
        generali
            .dati_bollo
            .as_ref()
            .expect("the stamp duty is present")
            .content
            .importo_bollo,
        Some(amount("2.00"))
    );
}

/// The same invoice built with the `cross_border_services` constructor and
/// its foreign currency equals the hand-checked fixture.
#[test]
fn td01_non_eu_us_client_priced_in_usd_matches_the_constructor() {
    let fixture = fixture_round_trips("td01-7ter-non-eu-usd.xml");
    let doc = Invoice::cross_border_services(
        &support::seller(),
        &support::business_us(),
        seven_ter_line("SERVIZIO DI CONSULENZA", "150.00"),
    )
    .progressivo("00003")
    .date("2026-11-04")
    .numero("00011")
    .causali(vec![
        "CONSULENZA AI SENSI DELL'ART. 7-TER DPR 633/72"
            .parse()
            .expect("a valid causale"),
    ])
    .currency(InvoiceCurrency::Foreign {
        currency: Currency::USD,
        rate: "1.08".parse().expect("a valid rate"),
        rate_date: "2026-11-04".parse().expect("a valid date"),
    })
    .build()
    .expect("the invoice passes every check");
    assert_eq!(doc, fixture);
}

/// Fixture 4: the `TD17` self-invoice for a service bought from an EU
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
    // The buyer transmits the self-invoice.
    assert_eq!(
        header
            .dati_trasmissione
            .content
            .id_trasmittente
            .content
            .id_codice,
        "04444444444"
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
    let pagamento = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_pagamento;
    assert_eq!(
        pagamento
            .first()
            .expect("a payment is present")
            .content
            .dettaglio_pagamento
            .first()
            .expect("a payment detail is present")
            .content
            .data_scadenza_pagamento,
        Some("2026-12-05".parse().expect("a valid date"))
    );
}

/// The same self-invoice built with the `SelfInvoice::td17` constructor
/// equals the hand-checked fixture.
#[test]
fn td17_eu_supplier_matches_the_constructor() {
    let fixture = fixture_round_trips("td17-eu-supplier.xml");
    let doc = td17_build(
        &support::business_de(),
        &support::buyer_company(),
        taxed_line("CONSULENZA COMMERCIALE PRESTATO IN GERMANIA", "100.00"),
        "00001",
        "2026-11-05",
        "001",
    );
    assert_eq!(doc, fixture);
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

/// Fixture 5: the `TD17` self-invoice for a service bought from a non-EU
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

/// The same self-invoice built with the `SelfInvoice::td17` constructor
/// equals the hand-checked fixture.
#[test]
fn td17_non_eu_supplier_matches_the_constructor() {
    let fixture = fixture_round_trips("td17-non-eu-supplier.xml");
    let doc = td17_build(
        &support::business_us(),
        &support::buyer_company(),
        taxed_line("CONSULENZA INFORMATIVA PRESTATO OLTRE OCEANO", "200.00"),
        "00002",
        "2026-11-06",
        "002",
    );
    assert_eq!(doc, fixture);
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

/// The `SelfInvoice::td17` form shared by the two reconstruction tests,
/// with the scenario progressivo, date and numero.
fn td17_build(
    supplier: &Business,
    buyer: &Company,
    line: Line,
    progressivo: &str,
    date: &str,
    numero: &str,
) -> FatturaElettronica {
    fatturapa::invoice::SelfInvoice::td17(
        supplier,
        buyer,
        Recipient::Codice("ABC1234".parse().expect("a valid code")),
        line,
    )
    .progressivo(progressivo)
    .date(date)
    .numero(numero)
    .causali(vec![
        "AUTOFATTURA PER SERVIZI ACQUISITI DALL'ESTERO, AI SENSI DELL'ART. 7-TER DPR 633/72"
            .parse()
            .expect("a valid causale"),
    ])
    .build()
    .expect("the self-invoice passes every check")
}

/// The domestic invoice with one line, its scenario progressivo, date
/// and numero.
fn domestic_invoice(line: Line) -> FatturaElettronica {
    Invoice::domestic(
        &support::seller(),
        &support::buyer(),
        Recipient::Codice("ABC1234".parse().expect("a valid code")),
        line,
    )
    .progressivo("00001")
    .date("2026-11-04")
    .numero("00013")
    .build()
    .expect("the invoice passes every check")
}

/// Every current `Natura` code is written into the `Natura` elements of
/// the document of the non-taxable line that carries it, and no other
/// code is written.
#[test]
fn every_natura_writes_its_code_in_the_document() {
    for natura in Natura::ALL {
        let line = Line::builder()
            .description("OPERAZIONE NON TASSABILE")
            .quantity("1.00")
            .unit_price("100.00")
            .vat(VatTreatment::NotTaxed {
                natura,
                riferimento_normativo: "ART. 7-TER DPR 633/72".parse().expect("a valid reference"),
            })
            .build()
            .expect("the line builds");
        let written = domestic_invoice(line)
            .to_xml_vec()
            .expect("the invoice serialises");
        let element = format!("<Natura>{}</Natura>", natura.code());
        assert!(
            written
                .windows(element.len())
                .any(|w| w == element.as_bytes()),
            "the {} code is written in the document",
            natura.code()
        );
        for other in Natura::ALL {
            if other == natura {
                continue;
            }
            let other_element = format!("<Natura>{}</Natura>", other.code());
            assert!(
                !written
                    .windows(other_element.len())
                    .any(|w| w == other_element.as_bytes()),
                "the {} code is not written for {}",
                other.code(),
                natura.code()
            );
        }
    }
}

/// Every `RegimeFiscale` code is written into the `RegimeFiscale`
/// element of the seller that carries it, and no other code is written.
#[test]
fn every_regime_fiscale_writes_its_code_in_the_document() {
    for regime in RegimeFiscale::ALL {
        let seller = Seller::denomination(
            "ALPHA SRL".parse().expect("a valid denomination"),
            "02222222222".parse().expect("a valid partita IVA"),
            Address::italian(
                "VIA ROMA 1".parse().expect("a valid address"),
                "00100".parse().expect("a valid CAP"),
                "ROMA".parse().expect("a valid comune"),
                Provincia::Rm,
            ),
            regime,
        );
        let written = Invoice::domestic(
            &seller,
            &support::buyer(),
            Recipient::Codice("ABC1234".parse().expect("a valid code")),
            taxed_line("SERVIZIO DI CONSULENZA", "100.00"),
        )
        .progressivo("00001")
        .date("2026-11-04")
        .numero("00013")
        .build()
        .expect("the invoice passes every check")
        .to_xml_vec()
        .expect("the invoice serialises");
        let element = format!("<RegimeFiscale>{}</RegimeFiscale>", regime.code());
        assert!(
            written
                .windows(element.len())
                .any(|w| w == element.as_bytes()),
            "the {} code is written in the document",
            regime.code()
        );
        for other in RegimeFiscale::ALL {
            if other == regime {
                continue;
            }
            let other_element = format!("<RegimeFiscale>{}</RegimeFiscale>", other.code());
            assert!(
                !written
                    .windows(other_element.len())
                    .any(|w| w == other_element.as_bytes()),
                "the {} code is not written for {}",
                other.code(),
                regime.code()
            );
        }
    }
}
