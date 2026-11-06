//! The ordinary invoice builder: totals, rounding, stamp duty, foreign
//! buyers and the `validate` run at `build` time.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

#[path = "support/mod.rs"]
mod support;

use fatturapa::builder::{BuildError, Invoice, InvoiceBody, Line};
use fatturapa::generated::fpr12 as g;
use fatturapa::{Amount, Date, Document, FatturaElettronica};

fn amount(text: &str) -> Amount {
    text.parse().expect("a valid decimal")
}

fn date(year: i32, month: u32, day: u32) -> Date {
    format!("{year:04}-{month:02}-{day:02}")
        .parse()
        .expect("a valid date")
}

fn seller() -> g::CedentePrestatoreType {
    g::CedentePrestatoreType::new(
        g::IdFiscaleType::new("IT", "01234567890"),
        g::AnagraficaType::denominazione("ALPHA SRL"),
        g::RegimeFiscaleType::Rf01,
        g::IndirizzoType::new("VIA ROMA 1", "00100", "ROMA", "IT"),
    )
}

/// An art. 7-ter services invoice to a US client. The client is non-resident,
/// so the builder fills `CodiceDestinatario` with `XXXXXXX` and the line
/// carries the `N2.1` nature with a zero rate. The total is above 77.47 EUR,
/// so the virtual stamp duty applies with its 2.00 EUR amount.
#[test]
fn art_7_ter_services_invoice_to_a_us_client() {
    let buyer = g::CessionarioCommittenteType::new(
        g::IdFiscaleType::new("US", "US123456789"),
        g::AnagraficaType::denominazione("ACME INC"),
        g::IndirizzoType::new("FIFTH AVENUE 1", "10001", "NEW YORK", "US"),
    );
    let doc = Invoice::new(g::FormatoTrasmissioneType::Fpr12, "00001", seller(), buyer)
        .body(
            InvoiceBody::new(
                g::TipoDocumentoType::Td01,
                "EUR",
                date(2026, 11, 4),
                "00012",
            )
            .causale("CONSULENZA AI SENSI DELL'ART. 7-TER DPR 633/72")
            .bollo_virtuale()
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

    assert_eq!(doc.versione, g::FormatoTrasmissioneType::Fpr12);
    assert_eq!(
        doc.content
            .fattura_elettronica_header
            .content
            .dati_trasmissione
            .content
            .codice_destinatario,
        "XXXXXXX"
    );
    let riep = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dati_riepilogo[0]
        .content;
    assert_eq!(riep.aliquota_iva, amount("0.00"));
    assert_eq!(riep.natura, Some(g::NaturaType::N21));
    assert_eq!(riep.imponibile_importo, amount("150.00"));
    assert_eq!(riep.imposta, amount("0.00"));
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

    let bytes = doc.to_xml_vec().expect("the invoice serialises");
    support::validate(&support::fpr12_schema(), &bytes)
        .expect("the written invoice validates against the schema");
    let reread = FatturaElettronica::from_xml_slice(&bytes).expect("the invoice re-reads");
    assert_eq!(reread, doc);
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
    let buyer = g::CessionarioCommittenteType::new(
        g::IdFiscaleType::new("IT", "09876543210"),
        g::AnagraficaType::denominazione("BETA SRL"),
        g::IndirizzoType::new("VIA MILANO 2", "20100", "MILANO", "IT"),
    );
    let doc = Invoice::new(g::FormatoTrasmissioneType::Fpr12, "00001", seller(), buyer)
        .codice_destinatario("ABC1234")
        .body(
            InvoiceBody::new(
                g::TipoDocumentoType::Td01,
                "EUR",
                date(2026, 11, 4),
                "00013",
            )
            .line(
                Line::new(1, "MATERIALE UFFICIO", amount("10.00"), amount("22.00"))
                    .quantita(amount("3.00")),
            )
            .line(Line::new(2, "CARTA", amount("5.55"), amount("22.00")).quantita(amount("3.00")))
            .line(
                Line::new(3, "FORNITURA CON SCONTO", amount("100.00"), amount("22.00"))
                    .sconto_maggiorazione(g::ScontoMaggiorazioneType::percentuale(
                        g::TipoScontoMaggiorazioneType::Sc,
                        amount("5.00"),
                    ))
                    .quantita(amount("3.00")),
            )
            .line(
                Line::new(
                    4,
                    "COMPONENTE A PREZZO UNITARIO FRAZIONARIO",
                    amount("0.333333"),
                    amount("22.00"),
                )
                .quantita(amount("3.00")),
            )
            .line(
                Line::new(5, "SALDAZIONE IVA 10", amount("1.11"), amount("10.00"))
                    .quantita(amount("2.00")),
            ),
        )
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

/// A virtual stamp duty on a total below 77.47 EUR is present without the
/// amount.
#[test]
fn stamp_duty_below_the_threshold_omits_the_amount() {
    let buyer = g::CessionarioCommittenteType::new(
        g::IdFiscaleType::new("IT", "09876543210"),
        g::AnagraficaType::denominazione("BETA SRL"),
        g::IndirizzoType::new("VIA MILANO 2", "20100", "MILANO", "IT"),
    );
    let doc = Invoice::new(g::FormatoTrasmissioneType::Fpr12, "00001", seller(), buyer)
        .codice_destinatario("ABC1234")
        .body(
            InvoiceBody::new(
                g::TipoDocumentoType::Td01,
                "EUR",
                date(2026, 11, 4),
                "00014",
            )
            .bollo_virtuale()
            .line(
                Line::new(1, "FORNITURA", amount("50.00"), amount("22.00"))
                    .quantita(amount("1.00")),
            ),
        )
        .build()
        .expect("the invoice passes every check");

    let generali = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content;
    let bollo = generali
        .dati_bollo
        .as_ref()
        .expect("the stamp duty is present");
    assert_eq!(bollo.content.bollo_virtuale, g::BolloVirtualeType::Si);
    assert_eq!(bollo.content.importo_bollo, None);
}

/// The stamp duty applies strictly above 77.47, so a total of exactly 77.47
/// omits the 2.00 amount.
#[test]
fn stamp_duty_applies_strictly_above_the_threshold() {
    for price in ["77.47", "77.48"] {
        let doc = Invoice::new(
            g::FormatoTrasmissioneType::Fpr12,
            "00001",
            seller(),
            buyer(),
        )
        .codice_destinatario("ABC1234")
        .body(
            InvoiceBody::new(
                g::TipoDocumentoType::Td01,
                "EUR",
                date(2026, 11, 4),
                "00021",
            )
            .bollo_virtuale()
            .line(
                Line::new(1, "FORNITURA ESCLUSA", amount(price), amount("0.00"))
                    .natura(g::NaturaType::N21)
                    .quantita(amount("1.00")),
            ),
        )
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
        assert_eq!(
            bollo.content.importo_bollo,
            (price == "77.48").then_some(amount("2.00"))
        );
    }
}

/// The line `PrezzoTotale` applies several `ScontoMaggiorazione` blocks in
/// row on the progressively adjusted unit price.
#[test]
fn line_total_applies_several_sconto_maggiorazione_blocks() {
    let doc = Invoice::new(
        g::FormatoTrasmissioneType::Fpr12,
        "00001",
        seller(),
        buyer(),
    )
    .codice_destinatario("ABC1234")
    .body(
        InvoiceBody::new(
            g::TipoDocumentoType::Td01,
            "EUR",
            date(2026, 11, 4),
            "00022",
        )
        .line(
            Line::new(
                1,
                "FORNITURA CON SCONTO E MAGGIORAZIONE",
                amount("100.00"),
                amount("22.00"),
            )
            .sconto_maggiorazione(g::ScontoMaggiorazioneType::percentuale(
                g::TipoScontoMaggiorazioneType::Sc,
                amount("10.00"),
            ))
            .sconto_maggiorazione(g::ScontoMaggiorazioneType::importo(
                g::TipoScontoMaggiorazioneType::Mg,
                amount("10.00"),
            ))
            .quantita(amount("1.00")),
        ),
    )
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

/// Without the flag no `DatiBollo` appears, even above the threshold.
#[test]
fn no_dati_bollo_without_the_flag() {
    let buyer = g::CessionarioCommittenteType::new(
        g::IdFiscaleType::new("IT", "09876543210"),
        g::AnagraficaType::denominazione("BETA SRL"),
        g::IndirizzoType::new("VIA MILANO 2", "20100", "MILANO", "IT"),
    );
    let doc = Invoice::new(g::FormatoTrasmissioneType::Fpr12, "00001", seller(), buyer)
        .codice_destinatario("ABC1234")
        .body(
            InvoiceBody::new(
                g::TipoDocumentoType::Td01,
                "EUR",
                date(2026, 11, 4),
                "00015",
            )
            .line(
                Line::new(1, "FORNITURA", amount("100.00"), amount("22.00"))
                    .quantita(amount("1.00")),
            ),
        )
        .build()
        .expect("the invoice passes every check");

    let generali = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content;
    assert_eq!(generali.dati_bollo, None);
}

/// A domestic buyer without an explicit `CodiceDestinatario` cannot be
/// assembled.
#[test]
fn domestic_buyer_requires_codice_destinatario() {
    let buyer = g::CessionarioCommittenteType::new(
        g::IdFiscaleType::new("IT", "09876543210"),
        g::AnagraficaType::denominazione("BETA SRL"),
        g::IndirizzoType::new("VIA MILANO 2", "20100", "MILANO", "IT"),
    );
    let error = Invoice::new(g::FormatoTrasmissioneType::Fpr12, "00001", seller(), buyer)
        .body(
            InvoiceBody::new(
                g::TipoDocumentoType::Td01,
                "EUR",
                date(2026, 11, 4),
                "00016",
            )
            .line(
                Line::new(1, "FORNITURA", amount("10.00"), amount("22.00"))
                    .quantita(amount("1.00")),
            ),
        )
        .build()
        .expect_err("the codice destinatario is missing");
    match error {
        BuildError::Missing(missing) => {
            assert_eq!(missing, vec!["codice_destinatario".to_string()]);
        }
        BuildError::Violations(violations) => {
            panic!("expected the missing field error, got {violations:?}")
        }
    }
}

/// A foreign buyer gets `XXXXXXX` without an explicit code, and the same
/// code with the explicit setter is kept.
#[test]
fn foreign_buyer_gets_xxxxx() {
    let us = g::CessionarioCommittenteType::new(
        g::IdFiscaleType::new("US", "US123456789"),
        g::AnagraficaType::denominazione("ACME INC"),
        g::IndirizzoType::new("FIFTH AVENUE 1", "10001", "NEW YORK", "US"),
    );
    let body = InvoiceBody::new(
        g::TipoDocumentoType::Td01,
        "EUR",
        date(2026, 11, 4),
        "00017",
    )
    .line(Line::new(1, "FORNITURA", amount("100.00"), amount("22.00")).quantita(amount("1.00")));
    let doc = Invoice::new(g::FormatoTrasmissioneType::Fpr12, "00001", seller(), us)
        .body(body)
        .build()
        .expect("the invoice passes every check");
    assert_eq!(
        doc.content
            .fattura_elettronica_header
            .content
            .dati_trasmissione
            .content
            .codice_destinatario,
        "XXXXXXX"
    );
}

/// `build` runs `validate` and returns every violation, here the zero rate
/// without a `Natura` in the line and in the riepilogo the builder computes.
#[test]
fn build_runs_validate_and_returns_all_violations() {
    let buyer = g::CessionarioCommittenteType::new(
        g::IdFiscaleType::new("IT", "09876543210"),
        g::AnagraficaType::denominazione("BETA SRL"),
        g::IndirizzoType::new("VIA MILANO 2", "20100", "MILANO", "IT"),
    );
    let error = Invoice::new(g::FormatoTrasmissioneType::Fpr12, "00001", seller(), buyer)
        .codice_destinatario("ABC1234")
        .body(
            InvoiceBody::new(
                g::TipoDocumentoType::Td01,
                "EUR",
                date(2026, 11, 4),
                "00018",
            )
            .line(
                Line::new(1, "FORNITURA ESCLUSA", amount("100.00"), amount("0.00"))
                    .quantita(amount("1.00")),
            ),
        )
        .build()
        .expect_err("a zero rate line without a natura fails the checks");
    match error {
        BuildError::Violations(violations) => {
            for code in ["00400", "00429"] {
                assert!(
                    violations.iter().any(|v| v.code == code),
                    "expected {code}, got {violations:?}"
                );
            }
        }
        BuildError::Missing(missing) => {
            panic!("expected the violations error, got {missing:?}")
        }
    }
}

/// An `FPA12` invoice with a 6 character codice destinatario assembles and
/// validates.
#[test]
fn fpa12_invoice_with_a_six_character_code() {
    let doc = Invoice::new(
        g::FormatoTrasmissioneType::Fpa12,
        "00001",
        seller(),
        buyer(),
    )
    .codice_destinatario("AAAAAA")
    .body(
        InvoiceBody::new(
            g::TipoDocumentoType::Td01,
            "EUR",
            date(2026, 11, 4),
            "00019",
        )
        .line(
            Line::new(1, "FORNITURA", amount("100.00"), amount("22.00")).quantita(amount("1.00")),
        ),
    )
    .build()
    .expect("the invoice passes every check");
    assert_eq!(doc.versione, g::FormatoTrasmissioneType::Fpa12);

    let bytes = doc.to_xml_vec().expect("the invoice serialises");
    support::validate(&support::fpr12_schema(), &bytes)
        .expect("the written invoice validates against the schema");

    let error = Invoice::new(
        g::FormatoTrasmissioneType::Fpa12,
        "00001",
        seller(),
        buyer(),
    )
    .codice_destinatario("AAAAAAA")
    .body(
        InvoiceBody::new(
            g::TipoDocumentoType::Td01,
            "EUR",
            date(2026, 11, 4),
            "00019",
        )
        .line(
            Line::new(1, "FORNITURA", amount("100.00"), amount("22.00")).quantita(amount("1.00")),
        ),
    )
    .build()
    .expect_err("a 7 character code in FPA12 fails 00427");
    match error {
        BuildError::Violations(violations) => {
            assert!(
                violations.iter().any(|v| v.code == "00427"),
                "{violations:?}"
            );
        }
        BuildError::Missing(missing) => {
            panic!("expected the violations error, got {missing:?}")
        }
    }
}

/// A `DatiRiepilogo` per distinct rate and nature keeps its insertion order.
#[test]
fn riepilogo_order_follows_the_lines() {
    let buyer = g::CessionarioCommittenteType::new(
        g::IdFiscaleType::new("IT", "09876543210"),
        g::AnagraficaType::denominazione("BETA SRL"),
        g::IndirizzoType::new("VIA MILANO 2", "20100", "MILANO", "IT"),
    );
    let doc = Invoice::new(g::FormatoTrasmissioneType::Fpr12, "00001", seller(), buyer)
        .codice_destinatario("ABC1234")
        .body(
            InvoiceBody::new(
                g::TipoDocumentoType::Td01,
                "EUR",
                date(2026, 11, 4),
                "00020",
            )
            .line(
                Line::new(1, "COMPONENTE 22", amount("10.00"), amount("22.00"))
                    .quantita(amount("1.00")),
            )
            .line(
                Line::new(2, "COMPONENTE 10", amount("20.00"), amount("10.00"))
                    .quantita(amount("1.00")),
            )
            .line(
                Line::new(3, "COMPONENTE 22 BIS", amount("30.00"), amount("22.00"))
                    .quantita(amount("1.00")),
            ),
        )
        .build()
        .expect("the invoice passes every check");

    let rie = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dati_riepilogo;
    assert_eq!(rie[0].content.aliquota_iva, amount("22.00"));
    assert_eq!(rie[0].content.imponibile_importo, amount("40.00"));
    assert_eq!(rie[1].content.aliquota_iva, amount("10.00"));
    assert_eq!(rie[1].content.imponibile_importo, amount("20.00"));
}

fn buyer() -> g::CessionarioCommittenteType {
    g::CessionarioCommittenteType::new(
        g::IdFiscaleType::new("IT", "09876543210"),
        g::AnagraficaType::denominazione("BETA SRL"),
        g::IndirizzoType::new("VIA MILANO 2", "20100", "MILANO", "IT"),
    )
}

/// A `RiferimentoNormativo` set for one nature lands only on the riepilogo
/// of that nature, and a taxable riepilogo carries none.
#[test]
fn riferimento_normativo_goes_to_its_nature_only() {
    let buyer = g::CessionarioCommittenteType::new(
        g::IdFiscaleType::new("IT", "09876543210"),
        g::AnagraficaType::denominazione("BETA SRL"),
        g::IndirizzoType::new("VIA MILANO 2", "20100", "MILANO", "IT"),
    );
    let doc = Invoice::new(g::FormatoTrasmissioneType::Fpr12, "00001", seller(), buyer)
        .codice_destinatario("ABC1234")
        .body(
            InvoiceBody::new(
                g::TipoDocumentoType::Td01,
                "EUR",
                date(2026, 11, 4),
                "00021",
            )
            .riferimento_normativo(g::NaturaType::N21, "ART. 7-TER DPR 633/72")
            .riferimento_normativo(g::NaturaType::N4, "ART. 10 DPR 633/72")
            .line(
                Line::new(1, "SERVIZIO TASSATO", amount("10.00"), amount("22.00"))
                    .quantita(amount("1.00")),
            )
            .line(
                Line::new(2, "SERVIZIO ESTERO", amount("20.00"), amount("0.00"))
                    .natura(g::NaturaType::N21)
                    .quantita(amount("1.00")),
            ),
        )
        .build()
        .expect("the invoice passes every check");

    let rie = &doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dati_riepilogo;
    assert_eq!(rie.len(), 2);
    assert_eq!(rie[0].content.riferimento_normativo, None);
    assert_eq!(rie[1].content.natura, Some(g::NaturaType::N21));
    assert_eq!(
        rie[1].content.riferimento_normativo.as_deref(),
        Some("ART. 7-TER DPR 633/72")
    );
}
