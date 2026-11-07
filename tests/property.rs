//! The property of the typed construction API. The scenario constructors
//! cannot express a violation of the `SDI` checks or of the schema, so
//! every generated combination of scenario, lines, currency and recipient
//! builds without error, passes `validate` and validates against the
//! official XSD.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

#[path = "support/mod.rs"]
mod support;

use fatturapa::ids::{Natura, Provincia, RegimeFiscale};
use fatturapa::invoice::{
    Invoice, InvoiceCurrency, Line, Recipient, Sconto, SelfInvoice, SevenTerLine, VatTreatment,
};
use fatturapa::party::{Address, Business, Buyer, Company, Seller};
use fatturapa::validate;
use fatturapa::{Document, FatturaElettronica};
use iso_country::Country;
use iso_currency::Currency;
use proptest::prelude::*;
use rust_decimal::Decimal;

/// The text of a value in cents, with its two decimals.
fn cents(value: u32) -> String {
    format!("{}.{:02}", value / 100, value % 100)
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

fn buyer_us() -> Business {
    Business::foreign(
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
    )
}

fn buyer_de() -> Business {
    Business::eu(
        fatturapa::ids::EuCountry::De,
        "MUSTER GMBH".parse().expect("a valid denomination"),
        "DE998877665".parse().expect("a valid identifier"),
        Address::foreign(
            "BEISPIELSTRASSE 1".parse().expect("a valid address"),
            Some("10115".parse().expect("a valid postal code")),
            "BERLIN".parse().expect("a valid comune"),
            Country::DE,
        )
        .expect("a valid foreign seat"),
    )
}

fn buyer_company() -> Company {
    Company::denomination(
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

fn supplier_de() -> Business {
    buyer_de()
}

fn supplier_us() -> Business {
    buyer_us()
}

/// The rate of a line, the two bits choosing among the three rates.
const fn rate(first: bool, second: bool) -> &'static str {
    match (first, second) {
        (true, true) => "22.00",
        (true, false) => "10.00",
        _ => "5.00",
    }
}

/// The `VatTreatment` of a line, a rate when `taxed` and the `N2.1` nature
/// with its reference otherwise.
fn treatment(taxed: bool, rate: &str) -> VatTreatment {
    if taxed {
        VatTreatment::Taxed(rate.parse().expect("a valid rate"))
    } else {
        VatTreatment::NotTaxed {
            natura: Natura::N2_1,
            riferimento_normativo: "ART. 7-TER DPR 633/72".parse().expect("a valid reference"),
        }
    }
}

/// The `ScontoMaggiorazione` blocks of a line, the 10 percent discount
/// when `discounted` and none otherwise.
fn sconti(discounted: bool) -> Vec<Sconto> {
    if discounted {
        vec![
            Sconto::discount_percent("10.00".parse::<Decimal>().expect("a valid decimal"))
                .expect("a valid discount"),
        ]
    } else {
        Vec::new()
    }
}

/// The line of the domestic invoice and of the self-invoice.
fn line(
    description: &str,
    price: u32,
    quantity: u32,
    rate: &str,
    taxed: bool,
    discounted: bool,
) -> Line {
    Line::builder()
        .description(description.parse().expect("a valid description"))
        .quantity(cents(quantity).parse().expect("a valid decimal"))
        .unit_price(cents(price).parse().expect("a valid decimal"))
        .sconti(sconti(discounted))
        .vat(treatment(taxed, rate))
        .build()
}

/// The line of the art. 7-ter invoice, without the VAT treatment the
/// scenario writes.
fn seven_ter_line(description: &str, price: u32, quantity: u32, discounted: bool) -> SevenTerLine {
    SevenTerLine::builder()
        .description(description.parse().expect("a valid description"))
        .quantity(cents(quantity).parse().expect("a valid decimal"))
        .unit_price(cents(price).parse().expect("a valid decimal"))
        .sconti(sconti(discounted))
        .build()
}

/// Builds the document of the scenario, checks it against `validate` and
/// validates its written bytes against the ordinary invoice schema.
fn check(doc: &FatturaElettronica) {
    let violations = validate::validate(doc);
    assert!(
        violations.is_empty(),
        "the typed invoice passes every check: {violations:?}"
    );
    let bytes = doc.to_xml_vec().expect("the invoice serialises");
    support::validate(&support::fpr12_schema(), &bytes)
        .expect("the invoice validates against the schema");
}

proptest! {
    #[test]
    fn any_typed_invoice_builds_and_passes_the_checks(
        scenario in 0u8..4,
        recipient in 0u8..3,
        currency in 0u8..4,
        bollo in any::<bool>(),
        lines in prop::collection::vec(
            (
                1u32..=9_999_999,
                1u32..=50,
                any::<bool>(),
                any::<bool>(),
                any::<bool>(),
                any::<bool>(),
            ),
            1..=3,
        ),
        progressivo in prop::sample::select(&["00001", "A2B3C", "00002"]),
        numero in prop::sample::select(&["00001", "FAT-2026", "042"]),
    ) {
        let seller = seller();
        let currency = if currency == 3 {
            InvoiceCurrency::Foreign {
                currency: Currency::USD,
                rate: "1.08".parse().expect("a valid rate"),
                rate_date: "2026-11-04".parse().expect("a valid date"),
            }
        } else {
            InvoiceCurrency::Eur
        };
        let date = "2026-11-04".parse().expect("a valid date");
        let progressivo = progressivo.parse().expect("a valid progressivo");
        let numero = numero.parse().expect("a valid number");
        let descriptions = ["SERVIZIO 1", "MATERIALE 2", "FORNITURA 3"];
        match scenario {
            0 => {
                let recipient = match recipient {
                    0 => Recipient::Codice("ABC1234".parse().expect("a valid code")),
                    1 => Recipient::Pec("beta@pec.it".parse().expect("a valid pec")),
                    _ => Recipient::Unknown,
                };
                let first = &lines[0];
                let first = line(
                    descriptions[0 % descriptions.len()],
                    first.0,
                    first.1,
                    rate(first.2, first.3),
                    first.4,
                    first.5,
                );
                let rest = lines[1..]
                    .iter()
                    .enumerate()
                    .map(|(i, l)| {
                        line(
                            descriptions[i % descriptions.len()],
                            l.0,
                            l.1,
                            rate(l.2, l.3),
                            l.4,
                            l.5,
                        )
                    })
                    .collect::<Vec<_>>();
                let buyer = buyer();
                let invoice = Invoice::domestic(
                    &seller,
                    &buyer,
                    recipient,
                    first,
                )
                .lines(rest)
                .progressivo(progressivo)
                .date(date)
                .numero(numero)
                .currency(currency);
                let doc = if bollo {
                    invoice.bollo().build()
                } else {
                    invoice.build()
                }
                .expect("the typed invoice builds");
                check(&doc);
            }
            1 => {
                let client = if recipient == 0 {
                    buyer_us()
                } else {
                    buyer_de()
                };
                let first = seven_ter_line(
                    descriptions[0 % descriptions.len()],
                    lines[0].0,
                    lines[0].1,
                    lines[0].5,
                );
                let rest = lines[1..]
                    .iter()
                    .enumerate()
                    .map(|(i, l)| {
                        seven_ter_line(
                            descriptions[i % descriptions.len()],
                            l.0,
                            l.1,
                            l.5,
                        )
                    })
                    .collect::<Vec<_>>();
                let doc = Invoice::cross_border_services(&seller, &client, first)
                    .lines(rest)
                    .progressivo(progressivo)
                    .date(date)
                    .numero(numero)
                    .currency(currency)
                    .build()
                    .expect("the typed invoice builds");
                check(&doc);
            }
            2 => {
                let line = line(
                    descriptions[0 % descriptions.len()],
                    lines[0].0,
                    lines[0].1,
                    "22.00",
                    true,
                    lines[0].5,
                );
                let doc = SelfInvoice::td17(
                    &supplier_de(),
                    &buyer_company(),
                    Recipient::Codice("ABC1234".parse().expect("a valid code")),
                    line,
                )
                .progressivo(progressivo)
                .date(date)
                .numero(numero)
                .currency(currency)
                .build()
                .expect("the typed self-invoice builds");
                check(&doc);
            }
            _ => {
                let line = line(
                    descriptions[0 % descriptions.len()],
                    lines[0].0,
                    lines[0].1,
                    "22.00",
                    true,
                    lines[0].5,
                );
                let doc = SelfInvoice::td19(
                    &supplier_us(),
                    &buyer_company(),
                    Recipient::Codice("ABC1234".parse().expect("a valid code")),
                    line,
                )
                .progressivo(progressivo)
                .date(date)
                .numero(numero)
                .currency(currency)
                .build()
                .expect("the typed self-invoice builds");
                check(&doc);
            }
        }
    }
}
