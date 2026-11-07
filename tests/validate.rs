//! Every Elenco Controlli check the crate implements gets one document that
//! triggers it and one that passes it.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

#[path = "support/mod.rs"]
mod support;

use fatturapa::generated::fpr12 as g;
use fatturapa::generated::fsm10 as sg;
use fatturapa::ids::{Provincia, RegimeFiscale};
use fatturapa::invoice::{Invoice, Line, Recipient, VatTreatment};
use fatturapa::party::{Address, Buyer, Seller};
use fatturapa::validate::{Violation, validate, validate_simplified};
use fatturapa::{Amount, Date, FatturaElettronica, FatturaElettronicaSemplificata};

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

fn assert_absent(violations: &[Violation], code: &str) {
    assert!(
        !violations.iter().any(|v| v.code == code),
        "did not expect {code}, got {violations:?}"
    );
}

fn seller() -> Seller {
    Seller::denomination(
        "ALPHA SRL".parse().expect("a valid denomination"),
        "01234567897".parse().expect("a valid partita IVA"),
        Address::italian(
            "VIA ROMA 1".parse().expect("a valid address"),
            "00100".parse().expect("a valid CAP"),
            "ROMA".parse().expect("a valid comune"),
            Provincia::Rm,
        ),
        RegimeFiscale::Rf01,
    )
}

fn buyer_iva() -> Buyer {
    Buyer::denomination(
        "BETA SRL".parse().expect("a valid denomination"),
        "09876543217".parse().expect("a valid partita IVA"),
        Address::italian(
            "VIA MILANO 2".parse().expect("a valid address"),
            "20100".parse().expect("a valid CAP"),
            "MILANO".parse().expect("a valid comune"),
            Provincia::Mi,
        ),
    )
}

fn buyer_cf() -> Buyer {
    Buyer::consumer(
        "MARIO".parse().expect("a valid name"),
        "ROSSI".parse().expect("a valid name"),
        "12345678903".parse().expect("a valid tax code"),
        Address::italian(
            "VIA TORINO 3".parse().expect("a valid address"),
            "10100".parse().expect("a valid CAP"),
            "TORINO".parse().expect("a valid comune"),
            Provincia::To,
        ),
    )
}

fn base_line() -> Line {
    let line: Line = Line::builder()
        .description("SERVIZIO DI CONSULENZA")
        .quantity("2.00")
        .unit_price("100.00")
        .vat(VatTreatment::Taxed("22.00".parse().expect("a valid rate")))
        .build()
        .expect("the line builds");
    line
}

/// A valid domestic `TD01` invoice with one 22 percent line.
fn base() -> FatturaElettronica {
    Invoice::domestic(
        &seller(),
        &buyer_iva(),
        Recipient::Codice("ABC1234".parse().expect("a valid recipient code")),
        base_line(),
    )
    .progressivo("00001")
    .date(date(2026, 11, 4))
    .numero("42")
    .build()
    .expect("the base document passes every check")
}

fn line(doc: &mut FatturaElettronica) -> &mut g::DettaglioLineeTypeContent {
    &mut doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dettaglio_linee[0]
        .content
}

fn riep(doc: &mut FatturaElettronica) -> &mut g::DatiRiepilogoTypeContent {
    &mut doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dati_riepilogo[0]
        .content
}

/// 00424, shared by the line and the riepilogo rate checks. Applies
/// `change` to the base document and checks the code, then checks that
/// the base document passes.
fn rate_not_a_percentage(change: impl FnOnce(&mut FatturaElettronica)) {
    let mut doc = base();
    change(&mut doc);
    assert_has(&validate(&doc), "00424");

    let doc = base();
    assert_absent(&validate(&doc), "00424");
}

fn cassa(
    aliquota: &str,
    natura: Option<g::NaturaType>,
    ritenuta: bool,
) -> g::DatiCassaPrevidenzialeType {
    g::DatiCassaPrevidenzialeType {
        content: g::DatiCassaPrevidenzialeTypeContent {
            tipo_cassa: g::TipoCassaType::Tc01,
            al_cassa: amount("5.00"),
            importo_contributo_cassa: amount("50.00"),
            imponibile_cassa: None,
            aliquota_iva: amount(aliquota),
            ritenuta: ritenuta.then_some(g::RitenutaType::Si),
            natura,
            riferimento_amministrazione: None,
        },
    }
}

fn id_iva(paese: &str, codice: &str) -> g::IdFiscaleType {
    g::IdFiscaleType {
        content: g::IdFiscaleTypeContent {
            id_paese: paese.to_string(),
            id_codice: codice.to_string(),
        },
    }
}

const fn sconto_importo(
    tipo: g::TipoScontoMaggiorazioneType,
    importo: Amount,
) -> g::ScontoMaggiorazioneType {
    g::ScontoMaggiorazioneType {
        content: g::ScontoMaggiorazioneTypeContent {
            tipo,
            percentuale: None,
            importo: Some(importo),
        },
    }
}

const fn sconto_percentuale(
    tipo: g::TipoScontoMaggiorazioneType,
    percentuale: Amount,
) -> g::ScontoMaggiorazioneType {
    g::ScontoMaggiorazioneType {
        content: g::ScontoMaggiorazioneTypeContent {
            tipo,
            percentuale: Some(percentuale),
            importo: None,
        },
    }
}

fn dati_ritenuta() -> g::DatiRitenutaType {
    g::DatiRitenutaType {
        content: g::DatiRitenutaTypeContent {
            tipo_ritenuta: g::TipoRitenutaType::Rt01,
            importo_ritenuta: amount("10.00"),
            aliquota_ritenuta: amount("20.00"),
            causale_pagamento: g::CausalePagamentoType::A,
        },
    }
}

/// The base document passes every check the crate implements.
#[test]
fn base_document_passes() {
    assert!(validate(&base()).is_empty(), "{:?}", validate(&base()));
}

/// 00313: the `XXXXXXX` `CodiceDestinatario` of an invoice to a buyer
/// established in `IT` triggers it; the base code and the `0000000` code
/// of a domestic buyer pass.
#[test]
fn code_00313_xxxxxxx_with_an_italian_buyer() {
    let mut doc = base();
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "XXXXXXX".to_string();
    assert_has(&validate(&doc), "00313");

    let mut doc = base();
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "0000000".to_string();
    assert_absent(&validate(&doc), "00313");

    let doc = base();
    assert_absent(&validate(&doc), "00313");
}

/// 00313: the `XXXXXXX` `CodiceDestinatario` with a buyer `IdFiscaleIVA`
/// established outside `IT` passes, the case the value exists for.
#[test]
fn code_00313_xxxxxxx_with_a_foreign_buyer() {
    let mut doc = base();
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "XXXXXXX".to_string();
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .as_mut()
        .expect("the base buyer carries its VAT number")
        .content
        .id_paese = "DE".to_string();
    assert_absent(&validate(&doc), "00313");
}

/// 00313: the `XXXXXXX` `CodiceDestinatario` with a buyer that carries
/// only its `CodiceFiscale` triggers it, the buyer `IdFiscaleIVA` being
/// unvalued.
#[test]
fn code_00313_xxxxxxx_without_the_buyer_iva() {
    let mut doc = base();
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "XXXXXXX".to_string();
    let buyer = &mut doc
        .content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content;
    buyer.id_fiscale_iva = None;
    buyer.codice_fiscale = Some("AB123456789".to_string());
    assert_has(&validate(&doc), "00313");
    assert_absent(&validate(&doc), "00417");
}

/// 00400: a zero-rate line without a `Natura` triggers it; the same line
/// with a `Natura` passes.
#[test]
fn code_00400_zero_rate_line_without_natura() {
    let mut doc = base();
    line(&mut doc).aliquota_iva = amount("0.00");
    assert_has(&validate(&doc), "00400");

    let mut doc = base();
    let l = line(&mut doc);
    l.aliquota_iva = amount("0.00");
    l.natura = Some(g::NaturaType::N1);
    assert_absent(&validate(&doc), "00400");
}

/// 00401: a line with a `Natura` and a non-zero rate triggers it; the base
/// line without a `Natura` passes.
#[test]
fn code_00401_line_with_natura_and_rate() {
    let mut doc = base();
    line(&mut doc).natura = Some(g::NaturaType::N1);
    assert_has(&validate(&doc), "00401");

    let doc = base();
    assert_absent(&validate(&doc), "00401");
}

/// 00409: two bodies with the same number and year both being ordinary
/// invoices trigger it; the same pair with one credit note passes.
#[test]
fn code_00409_duplicate_number_in_the_document() {
    let mut doc = base();
    doc.content
        .fattura_elettronica_body
        .push(doc.content.fattura_elettronica_body[0].clone());
    assert_has(&validate(&doc), "00409");

    let mut doc = base();
    let mut body = doc.content.fattura_elettronica_body[0].clone();
    body.content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .tipo_documento = g::TipoDocumentoType::Td04;
    doc.content.fattura_elettronica_body.push(body);
    assert_absent(&validate(&doc), "00409");
}

/// 00411: a line with `Ritenuta SI` and no `DatiRitenuta` triggers it; the
/// same line with a `DatiRitenuta` block passes.
#[test]
fn code_00411_line_ritenuta_without_dati_ritenuta() {
    let mut doc = base();
    line(&mut doc).ritenuta = Some(g::RitenutaType::Si);
    assert_has(&validate(&doc), "00411");

    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_ritenuta
        .push(dati_ritenuta());
    line(&mut doc).ritenuta = Some(g::RitenutaType::Si);
    assert_absent(&validate(&doc), "00411");
}

/// 00413: a zero-rate `DatiCassaPrevidenziale` without a `Natura` triggers
/// it; the same block with a `Natura` passes.
#[test]
fn code_00413_cassa_zero_rate_without_natura() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("0.00", None, false));
    assert_has(&validate(&doc), "00413");

    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("0.00", Some(g::NaturaType::N4), false));
    assert_absent(&validate(&doc), "00413");
}

/// 00414: a `DatiCassaPrevidenziale` with a `Natura` and a non-zero rate
/// triggers it; the same block without a `Natura` passes.
#[test]
fn code_00414_cassa_with_natura_and_rate() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("10.00", Some(g::NaturaType::N4), false));
    assert_has(&validate(&doc), "00414");

    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("10.00", None, false));
    assert_absent(&validate(&doc), "00414");
}

/// 00415: a `DatiCassaPrevidenziale` with `Ritenuta SI` and no
/// `DatiRitenuta` triggers it; the same block with a `DatiRitenuta` passes.
#[test]
fn code_00415_cassa_ritenuta_without_dati_ritenuta() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("22.00", None, true));
    assert_has(&validate(&doc), "00415");

    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_ritenuta
        .push(dati_ritenuta());
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("22.00", None, true));
    assert_absent(&validate(&doc), "00415");
}

/// 00417: a buyer with neither `IdFiscaleIVA` nor `CodiceFiscale` triggers
/// it; a buyer with a VAT number passes.
#[test]
fn code_00417_buyer_without_fiscal_identifiers() {
    let doc = Invoice::domestic(
        &seller(),
        &buyer_cf(),
        Recipient::Codice("ABC1234".parse().expect("a valid recipient code")),
        base_line(),
    )
    .progressivo("00001")
    .date(date(2026, 11, 4))
    .numero("42")
    .build()
    .expect("a buyer identified by tax code assembles fine");
    assert_absent(&validate(&doc), "00417");

    let mut doc = base();
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
        .codice_fiscale = None;
    assert_has(&validate(&doc), "00417");
}

/// 00418: a credit note dated before the linked invoice triggers it; the
/// same note dated after the linked invoice passes.
#[test]
fn code_00418_credit_note_before_the_linked_invoice() {
    let mut doc = base();
    let generali = &mut doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content;
    generali.dati_generali_documento.content.tipo_documento = g::TipoDocumentoType::Td04;
    generali
        .dati_fatture_collegate
        .push(g::DatiDocumentiCorrelatiType {
            content: g::DatiDocumentiCorrelatiTypeContent {
                riferimento_numero_linea: Vec::new(),
                id_documento: "41".to_string(),
                data: Some(date(2026, 11, 5)),
                num_item: None,
                codice_commessa_convenzione: None,
                codice_cup: None,
                codice_cig: None,
            },
        });
    assert_has(&validate(&doc), "00418");

    let mut doc = base();
    let generali = &mut doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content;
    generali.dati_generali_documento.content.tipo_documento = g::TipoDocumentoType::Td04;
    generali
        .dati_fatture_collegate
        .push(g::DatiDocumentiCorrelatiType {
            content: g::DatiDocumentiCorrelatiTypeContent {
                riferimento_numero_linea: Vec::new(),
                id_documento: "41".to_string(),
                data: Some(date(2026, 11, 1)),
                num_item: None,
                codice_commessa_convenzione: None,
                codice_cup: None,
                codice_cig: None,
            },
        });
    assert_absent(&validate(&doc), "00418");
}

/// 00418: a non-`TD04` document with a linked invoice dated after the
/// document triggers it; the same document with an earlier linked date
/// passes.
#[test]
fn code_00418_linked_invoice_after_the_document() {
    let mut doc = base();
    let generali = &mut doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content;
    generali
        .dati_fatture_collegate
        .push(g::DatiDocumentiCorrelatiType {
            content: g::DatiDocumentiCorrelatiTypeContent {
                riferimento_numero_linea: Vec::new(),
                id_documento: "41".to_string(),
                data: Some(date(2026, 11, 10)),
                num_item: None,
                codice_commessa_convenzione: None,
                codice_cup: None,
                codice_cig: None,
            },
        });
    assert_has(&validate(&doc), "00418");

    let mut doc = base();
    let generali = &mut doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content;
    generali
        .dati_fatture_collegate
        .push(g::DatiDocumentiCorrelatiType {
            content: g::DatiDocumentiCorrelatiTypeContent {
                riferimento_numero_linea: Vec::new(),
                id_documento: "41".to_string(),
                data: Some(date(2026, 11, 1)),
                num_item: None,
                codice_commessa_convenzione: None,
                codice_cup: None,
                codice_cig: None,
            },
        });
    assert_absent(&validate(&doc), "00418");
}

/// 00419: two distinct rates and one `DatiRiepilogo` block trigger it; one
/// rate and one block pass.
#[test]
fn code_00419_rate_without_riepilogo() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dettaglio_linee
        .push(g::DettaglioLineeType {
            content: g::DettaglioLineeTypeContent {
                numero_linea: 2,
                tipo_cessione_prestazione: None,
                codice_articolo: Vec::new(),
                descrizione: "ALTRA FORNITURA".to_string(),
                quantita: Some(amount("1.00")),
                unita_misura: None,
                data_inizio_periodo: None,
                data_fine_periodo: None,
                prezzo_unitario: amount("10.00"),
                sconto_maggiorazione: Vec::new(),
                prezzo_totale: amount("10.00"),
                aliquota_iva: amount("10.00"),
                ritenuta: None,
                natura: None,
                riferimento_amministrazione: None,
                altri_dati_gestionali: Vec::new(),
            },
        });
    assert_has(&validate(&doc), "00419");

    let doc = base();
    assert_absent(&validate(&doc), "00419");
}

/// 00420: a `DatiRiepilogo` with `EsigibilitaIVA S` and an `N6` nature
/// triggers it; the same block without a nature passes.
#[test]
fn code_00420_reverse_charge_with_split_payment() {
    let mut doc = base();
    let r = riep(&mut doc);
    r.esigibilita_iva = Some(g::EsigibilitaIvaType::S);
    r.natura = Some(g::NaturaType::N6);
    assert_has(&validate(&doc), "00420");

    let mut doc = base();
    riep(&mut doc).esigibilita_iva = Some(g::EsigibilitaIvaType::S);
    assert_absent(&validate(&doc), "00420");
}

/// 00421: an `Imposta` farther than one cent from the rate applied to the
/// taxable amount triggers it; the base block passes.
#[test]
fn code_00421_imposta_mismatch() {
    let mut doc = base();
    riep(&mut doc).imposta = amount("0.00");
    assert_has(&validate(&doc), "00421");

    let doc = base();
    assert_absent(&validate(&doc), "00421");
}

/// 00422: an `ImponibileImporto` farther than one euro from the sum of the
/// line totals triggers it; the base block passes.
#[test]
fn code_00422_imponibile_mismatch() {
    let mut doc = base();
    riep(&mut doc).imponibile_importo = amount("0.00");
    assert_has(&validate(&doc), "00422");

    let doc = base();
    assert_absent(&validate(&doc), "00422");
}

/// 00423: a `PrezzoTotale` farther than one cent from the adjusted unit
/// price times the quantity triggers it, at the path of the line element;
/// the base line passes.
#[test]
fn code_00423_prezzo_totale_mismatch() {
    let mut doc = base();
    line(&mut doc).prezzo_totale = amount("0.00");
    let v = validate(&doc);
    assert_has(&v, "00423");
    let viol = v
        .iter()
        .find(|x| x.code == "00423")
        .expect("00423 is present");
    assert_eq!(
        viol.path,
        "FatturaElettronicaBody[1]/DatiBeniServizi/DettaglioLinee[1]/PrezzoTotale"
    );

    let doc = base();
    assert_absent(&validate(&doc), "00423");
}

/// 00424: a non-zero rate below 1.00 is not a percentage; the base rate
/// passes.
#[test]
fn code_00424_rate_not_a_percentage() {
    rate_not_a_percentage(|doc| {
        line(doc).aliquota_iva = amount("0.50");
    });
}

/// 00424: a `DatiRiepilogo` rate of 0.50 is not a percentage; the base
/// rate passes.
#[test]
fn code_00424_riepilogo_rate_not_a_percentage() {
    rate_not_a_percentage(|doc| {
        riep(doc).aliquota_iva = amount("0.50");
    });
}

/// 00425: a `Numero` without any digit triggers it; the base number passes.
#[test]
fn code_00425_number_without_digit() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .numero = "ABC".to_string();
    assert_has(&validate(&doc), "00425");

    let doc = base();
    assert_absent(&validate(&doc), "00425");
}

/// 00427: a 6 character `CodiceDestinatario` in `FPR12` and a 7 character
/// code in `FPA12` trigger it; the base 7 character code in `FPR12` passes.
#[test]
fn code_00427_codice_destinatario_length() {
    let mut doc = base();
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "ABC123".to_string();
    assert_has(&validate(&doc), "00427");

    let mut doc = base();
    doc.versione = g::FormatoTrasmissioneType::Fpa12;
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .formato_trasmissione = g::FormatoTrasmissioneType::Fpa12;
    assert_has(&validate(&doc), "00427");

    let doc = base();
    assert_absent(&validate(&doc), "00427");
}

/// 00428: `versione` and `FormatoTrasmissione` are the same enumeration, so
/// the check is guaranteed by the generated types and the base document
/// reports it absent.
#[test]
fn code_00428_versione_coherence() {
    let doc = base();
    assert_absent(&validate(&doc), "00428");
}

/// 00429: a zero-rate `DatiRiepilogo` without a `Natura` triggers it; the
/// same block with a `Natura` passes.
#[test]
fn code_00429_riepilogo_zero_rate_without_natura() {
    let mut doc = base();
    let r = riep(&mut doc);
    r.aliquota_iva = amount("0.00");
    assert_has(&validate(&doc), "00429");

    let mut doc = base();
    let r = riep(&mut doc);
    r.aliquota_iva = amount("0.00");
    r.natura = Some(g::NaturaType::N1);
    assert_absent(&validate(&doc), "00429");
}

/// 00430: a `DatiRiepilogo` with a `Natura` and a non-zero rate triggers it;
/// the base block passes.
#[test]
fn code_00430_riepilogo_with_natura_and_rate() {
    let mut doc = base();
    riep(&mut doc).natura = Some(g::NaturaType::N1);
    assert_has(&validate(&doc), "00430");

    let doc = base();
    assert_absent(&validate(&doc), "00430");
}

/// 00437: a document-level `ScontoMaggiorazione` with a `Tipo` and neither
/// `Percentuale` nor `Importo` triggers it; the same block with a
/// percentage passes.
#[test]
fn code_00437_document_sconto_without_value() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .sconto_maggiorazione
        .push(g::ScontoMaggiorazioneType {
            content: g::ScontoMaggiorazioneTypeContent {
                tipo: g::TipoScontoMaggiorazioneType::Sc,
                percentuale: None,
                importo: None,
            },
        });
    assert_has(&validate(&doc), "00437");

    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .sconto_maggiorazione
        .push(g::ScontoMaggiorazioneType {
            content: g::ScontoMaggiorazioneTypeContent {
                tipo: g::TipoScontoMaggiorazioneType::Sc,
                percentuale: Some(amount("5.00")),
                importo: None,
            },
        });
    assert_absent(&validate(&doc), "00437");
}

/// 00438: a line `ScontoMaggiorazione` with a `Tipo` and neither
/// `Percentuale` nor `Importo` triggers it; the same block with an amount
/// passes.
#[test]
fn code_00438_line_sconto_without_value() {
    let mut doc = base();
    line(&mut doc)
        .sconto_maggiorazione
        .push(g::ScontoMaggiorazioneType {
            content: g::ScontoMaggiorazioneTypeContent {
                tipo: g::TipoScontoMaggiorazioneType::Sc,
                percentuale: None,
                importo: None,
            },
        });
    assert_has(&validate(&doc), "00438");

    let mut doc = base();
    line(&mut doc)
        .sconto_maggiorazione
        .push(g::ScontoMaggiorazioneType {
            content: g::ScontoMaggiorazioneTypeContent {
                tipo: g::TipoScontoMaggiorazioneType::Sc,
                percentuale: None,
                importo: Some(amount("1.00")),
            },
        });
    line(&mut doc).prezzo_totale = amount("398.00");
    assert_absent(&validate(&doc), "00438");
}

/// 00443: a line rate absent from every `DatiRiepilogo` block triggers it;
/// the base line passes.
#[test]
fn code_00443_line_rate_without_riepilogo() {
    let mut doc = base();
    line(&mut doc).aliquota_iva = amount("10.00");
    assert_has(&validate(&doc), "00443");

    let doc = base();
    assert_absent(&validate(&doc), "00443");
}

/// 00444: a line `Natura` absent from every `DatiRiepilogo` triggers it;
/// the same nature in the riepilogo passes.
#[test]
fn code_00444_line_natura_without_riepilogo() {
    let mut doc = base();
    line(&mut doc).natura = Some(g::NaturaType::N1);
    assert_has(&validate(&doc), "00444");

    let mut doc = base();
    line(&mut doc).natura = Some(g::NaturaType::N1);
    riep(&mut doc).natura = Some(g::NaturaType::N1);
    assert_absent(&validate(&doc), "00444");
}

/// 00445: the generic nature `N2` is not allowed; the detailed nature
/// `N2.1` passes.
#[test]
fn code_00445_generic_natura() {
    let mut doc = base();
    line(&mut doc).natura = Some(g::NaturaType::N2);
    assert_has(&validate(&doc), "00445");

    let mut doc = base();
    line(&mut doc).natura = Some(g::NaturaType::N21);
    assert_absent(&validate(&doc), "00445");
}

/// 00445: the generic nature `N2` in a `DatiCassaPrevidenziale` is not
/// allowed; a detailed nature passes.
#[test]
fn code_00445_cassa_generic_natura() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("0.00", Some(g::NaturaType::N2), false));
    assert_has(&validate(&doc), "00445");

    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("0.00", Some(g::NaturaType::N21), false));
    assert_absent(&validate(&doc), "00445");
}

/// 00471: for `TD16` the seller and the buyer must differ; the same
/// document with a different buyer passes.
#[test]
fn code_00471_identical_parties_for_td16() {
    let mut doc = base();
    let generali = &mut doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content;
    generali.dati_generali_documento.content.tipo_documento = g::TipoDocumentoType::Td16;
    let buyer = &mut doc
        .content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content;
    buyer.id_fiscale_iva = Some(id_iva("IT", "01234567897"));
    assert_has(&validate(&doc), "00471");

    let doc = base();
    assert_absent(&validate(&doc), "00471");
}

/// 00472: for `TD21` the seller and the buyer must be identical; the base
/// document with a different buyer triggers it and the same buyer passes.
#[test]
fn code_00472_different_parties_for_td21() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .tipo_documento = g::TipoDocumentoType::Td21;
    assert_has(&validate(&doc), "00472");

    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .tipo_documento = g::TipoDocumentoType::Td21;
    let buyer = &mut doc
        .content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content;
    buyer.id_fiscale_iva = Some(id_iva("IT", "01234567897"));
    assert_absent(&validate(&doc), "00472");
}

/// 00471: for `TD01`, `TD02`, `TD03`, `TD06`, `TD24`, `TD25` and `TD28` the
/// seller and the buyer must differ; the internal transfer `TD26` stays
/// outside the check; the base document with a different buyer passes.
#[test]
fn code_00471_identical_parties_for_the_sale_and_transfer_types() {
    for tipo in [
        g::TipoDocumentoType::Td01,
        g::TipoDocumentoType::Td02,
        g::TipoDocumentoType::Td03,
        g::TipoDocumentoType::Td06,
        g::TipoDocumentoType::Td24,
        g::TipoDocumentoType::Td25,
        g::TipoDocumentoType::Td26,
        g::TipoDocumentoType::Td28,
    ] {
        let expect_00471 = tipo != g::TipoDocumentoType::Td26;
        let mut doc = base();
        doc.content.fattura_elettronica_body[0]
            .content
            .dati_generali
            .content
            .dati_generali_documento
            .content
            .tipo_documento = tipo;
        let buyer = &mut doc
            .content
            .fattura_elettronica_header
            .content
            .cessionario_committente
            .content
            .dati_anagrafici
            .content;
        buyer.id_fiscale_iva = Some(id_iva("IT", "01234567897"));
        if expect_00471 {
            assert_has(&validate(&doc), "00471");
        } else {
            assert_absent(&validate(&doc), "00471");
        }
    }

    let doc = base();
    assert_absent(&validate(&doc), "00471");
}

/// 00472: for `TD27` the seller and the buyer must be identical; the base
/// document with a different buyer triggers it and the same buyer passes.
#[test]
fn code_00472_different_parties_for_td27() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .tipo_documento = g::TipoDocumentoType::Td27;
    assert_has(&validate(&doc), "00472");

    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .tipo_documento = g::TipoDocumentoType::Td27;
    let buyer = &mut doc
        .content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content;
    buyer.id_fiscale_iva = Some(id_iva("IT", "01234567897"));
    assert_absent(&validate(&doc), "00472");
}

/// 00473: for `TD17` an Italian seller is not allowed and for `TD29` a
/// non-Italian seller is; a non-Italian seller with `TD17` passes.
#[test]
fn code_00473_seller_country_for_the_document_type() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .tipo_documento = g::TipoDocumentoType::Td17;
    assert_has(&validate(&doc), "00473");

    let mut doc = base();
    let generali = &mut doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content;
    generali.dati_generali_documento.content.tipo_documento = g::TipoDocumentoType::Td29;
    doc.content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .content
        .id_paese = "US".to_string();
    assert_has(&validate(&doc), "00473");

    let mut doc = base();
    let generali = &mut doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content;
    generali.dati_generali_documento.content.tipo_documento = g::TipoDocumentoType::Td17;
    doc.content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .content
        .id_paese = "US".to_string();
    assert_absent(&validate(&doc), "00473");
}

/// 00474: for `TD21` a zero-rate line is not allowed; the base 22 percent
/// line passes.
#[test]
fn code_00474_zero_rate_line_for_td21() {
    let mut doc = base();
    let generali = &mut doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content;
    generali.dati_generali_documento.content.tipo_documento = g::TipoDocumentoType::Td21;
    let l = line(&mut doc);
    l.aliquota_iva = amount("0.00");
    l.natura = Some(g::NaturaType::N1);
    assert_has(&validate(&doc), "00474");

    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .tipo_documento = g::TipoDocumentoType::Td21;
    assert_absent(&validate(&doc), "00474");
}

/// 00475: for `TD22` the buyer `IdFiscaleIVA` is required; a buyer with a
/// VAT number passes.
#[test]
fn code_00475_buyer_iva_required_for_td22() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .tipo_documento = g::TipoDocumentoType::Td22;
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva = None;
    assert_has(&validate(&doc), "00475");

    let doc = base();
    assert_absent(&validate(&doc), "00475");
}

/// 00476: a seller and a buyer whose `IdPaese` are both different from
/// `IT` trigger it; a buyer in `IT` passes.
#[test]
fn code_00476_both_parties_outside_italy() {
    let mut doc = base();
    doc.content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .content
        .id_paese = "US".to_string();
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .as_mut()
        .expect("the base buyer has a VAT number")
        .content
        .id_paese = "FR".to_string();
    assert_has(&validate(&doc), "00476");

    let mut doc = base();
    doc.content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva
        .content
        .id_paese = "US".to_string();
    assert_absent(&validate(&doc), "00476");
}

/// 00423: a line with an amount discount passes with the adjusted unit price
/// times the quantity.
#[test]
fn code_00423_discount_amount() {
    let mut doc = base();
    let l = line(&mut doc);
    l.sconto_maggiorazione.push(sconto_importo(
        g::TipoScontoMaggiorazioneType::Sc,
        amount("10.00"),
    ));
    l.prezzo_totale = amount("180.00");
    assert_absent(&validate(&doc), "00423");
}

/// 00423: a line with an amount markup passes with the adjusted unit price
/// times the quantity.
#[test]
fn code_00423_markup_amount() {
    let mut doc = base();
    let l = line(&mut doc);
    l.sconto_maggiorazione.push(sconto_importo(
        g::TipoScontoMaggiorazioneType::Mg,
        amount("10.00"),
    ));
    l.prezzo_totale = amount("220.00");
    assert_absent(&validate(&doc), "00423");
}

/// 00423: cascading percentages apply to the progressively adjusted unit
/// price.
#[test]
fn code_00423_cascading_percentages() {
    let mut doc = base();
    let l = line(&mut doc);
    l.sconto_maggiorazione.push(sconto_percentuale(
        g::TipoScontoMaggiorazioneType::Sc,
        amount("10.00"),
    ));
    l.sconto_maggiorazione.push(sconto_percentuale(
        g::TipoScontoMaggiorazioneType::Mg,
        amount("10.00"),
    ));
    l.prezzo_totale = amount("198.00");
    assert_absent(&validate(&doc), "00423");
}

/// 00423: a `PrezzoTotale` off by exactly one cent is a violation, and one
/// below it is not.
#[test]
fn code_00423_tolerance_boundary() {
    let mut doc = base();
    line(&mut doc).prezzo_totale = amount("200.01");
    assert_has(&validate(&doc), "00423");

    let mut doc = base();
    line(&mut doc).prezzo_totale = amount("199.995");
    assert_absent(&validate(&doc), "00423");
}

/// 00421: an `Imposta` off by exactly one cent is a violation, and one below
/// it is not.
#[test]
fn code_00421_tolerance_boundary() {
    let mut doc = base();
    riep(&mut doc).imposta = amount("44.01");
    assert_has(&validate(&doc), "00421");

    let mut doc = base();
    riep(&mut doc).imposta = amount("44.0050");
    assert_absent(&validate(&doc), "00421");
}

/// 00422: an `ImponibileImporto` off by exactly one euro is a violation, and
/// one below it is not.
#[test]
fn code_00422_tolerance_boundary() {
    let mut doc = base();
    riep(&mut doc).imponibile_importo = amount("199.00");
    assert_has(&validate(&doc), "00422");

    let mut doc = base();
    riep(&mut doc).imponibile_importo = amount("199.01");
    assert_absent(&validate(&doc), "00422");
}

/// 00422: a rate shared by the lines and a `DatiCassaPrevidenziale` block
/// reports once.
#[test]
fn code_00422_reports_once_per_rate() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("22.00", None, false));
    riep(&mut doc).imponibile_importo = amount("0.00");
    let v = validate(&doc);
    let count = v.iter().filter(|x| x.code == "00422").count();
    assert_eq!(count, 1, "{v:?}");
}

/// 00422: a rate carried only by a `DatiCassaPrevidenziale` block is checked.
#[test]
fn code_00422_cassa_only_rate() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("13.00", None, false));
    assert_has(&validate(&doc), "00422");
}

/// 00422: the cassa contributions and the `Arrotondamento` join the line
/// totals in the sum.
#[test]
fn code_00422_cassa_and_arrotondamento_included() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("22.00", None, false));
    let r = riep(&mut doc);
    r.imponibile_importo = amount("251.50");
    r.arrotondamento = Some(amount("1.50"));
    r.imposta = amount("55.33");
    assert_absent(&validate(&doc), "00422");
    assert_absent(&validate(&doc), "00421");
}

/// 00419: a cassa rate without a matching `DatiRiepilogo` block triggers it.
#[test]
fn code_00419_cassa_rate_without_riepilogo() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("10.00", None, false));
    assert_has(&validate(&doc), "00419");
}

/// 00443: a cassa rate that appears in a `DatiRiepilogo` block passes.
#[test]
fn code_00443_cassa_rate_in_riepilogo() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("22.00", None, false));
    assert_absent(&validate(&doc), "00443");
}

/// 00444: a cassa `Natura` that appears in a `DatiRiepilogo` block passes.
#[test]
fn code_00444_cassa_natura_in_riepilogo() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("0.00", Some(g::NaturaType::N1), false));
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi
        .content
        .dati_riepilogo
        .push(g::DatiRiepilogoType {
            content: g::DatiRiepilogoTypeContent {
                aliquota_iva: amount("0.00"),
                natura: Some(g::NaturaType::N1),
                spese_accessorie: None,
                arrotondamento: None,
                imponibile_importo: amount("0.00"),
                imposta: amount("0.00"),
                esigibilita_iva: None,
                riferimento_normativo: None,
            },
        });
    assert_absent(&validate(&doc), "00443");
    assert_absent(&validate(&doc), "00444");
}

/// 00424: a rate of exactly 1.00 is a percentage in the line and the
/// `DatiRiepilogo`.
#[test]
fn code_00424_rate_of_one_percent() {
    let mut doc = base();
    line(&mut doc).aliquota_iva = amount("1.00");
    let r = riep(&mut doc);
    r.aliquota_iva = amount("1.00");
    r.imposta = amount("2.00");
    assert_absent(&validate(&doc), "00424");
}

/// 00424: the cassa rate 0.50 triggers it, and 1.00 and 0.00 do not.
#[test]
fn code_00424_cassa_rate_boundaries() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("0.50", None, false));
    assert_has(&validate(&doc), "00424");

    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("1.00", None, false));
    assert_absent(&validate(&doc), "00424");

    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .dati_cassa_previdenziale
        .push(cassa("0.00", Some(g::NaturaType::N1), false));
    assert_absent(&validate(&doc), "00424");
}

/// 00420: an `EsigibilitaIVA S` with a nature outside the `N6` family
/// passes.
#[test]
fn code_00420_nature_outside_the_n6_family() {
    let mut doc = base();
    let r = riep(&mut doc);
    r.esigibilita_iva = Some(g::EsigibilitaIvaType::S);
    r.natura = Some(g::NaturaType::N21);
    assert_absent(&validate(&doc), "00420");
}

/// 00418: a credit note dated as its linked invoice passes.
#[test]
fn code_00418_credit_note_dated_as_the_linked_invoice() {
    let mut doc = base();
    let generali = &mut doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content;
    generali.dati_generali_documento.content.tipo_documento = g::TipoDocumentoType::Td04;
    generali
        .dati_fatture_collegate
        .push(g::DatiDocumentiCorrelatiType {
            content: g::DatiDocumentiCorrelatiTypeContent {
                riferimento_numero_linea: Vec::new(),
                id_documento: "41".to_string(),
                data: Some(date(2026, 11, 4)),
                num_item: None,
                codice_commessa_convenzione: None,
                codice_cup: None,
                codice_cig: None,
            },
        });
    assert_absent(&validate(&doc), "00418");
}

/// 00471: a `TD16` with a buyer different from the seller passes.
#[test]
fn code_00471_different_parties_for_td16() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .tipo_documento = g::TipoDocumentoType::Td16;
    assert_absent(&validate(&doc), "00471");
}

/// 00473: an Italian seller with `TD29` passes.
#[test]
fn code_00473_italian_seller_for_td29() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .tipo_documento = g::TipoDocumentoType::Td29;
    assert_absent(&validate(&doc), "00473");
}

/// 00409: with `Art73 SI` the check compares the full date, so the same
/// number on different dates in the same year passes.
#[test]
fn code_00409_art73_uses_the_full_date() {
    let mut doc = base();
    let mut body = doc.content.fattura_elettronica_body[0].clone();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .art_73 = Some(g::Art73Type::Si);
    body.content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .art_73 = Some(g::Art73Type::Si);
    body.content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .data = date(2026, 11, 5);
    doc.content.fattura_elettronica_body.push(body);
    assert_absent(&validate(&doc), "00409");
}

/// 00409: with `Art73 SI` the same number and the same full date duplicate.
#[test]
fn code_00409_art73_same_date_duplicates() {
    let mut doc = base();
    let mut body = doc.content.fattura_elettronica_body[0].clone();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .art_73 = Some(g::Art73Type::Si);
    body.content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .art_73 = Some(g::Art73Type::Si);
    doc.content.fattura_elettronica_body.push(body);
    assert_has(&validate(&doc), "00409");
}

/// 00409: without `Art73` the check compares the year, so the same number in
/// different years passes.
#[test]
fn code_00409_same_number_different_years() {
    let mut doc = base();
    let mut body = doc.content.fattura_elettronica_body[0].clone();
    body.content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .data = date(2025, 11, 4);
    doc.content.fattura_elettronica_body.push(body);
    assert_absent(&validate(&doc), "00409");
}

/// 00409: a credit note and an ordinary invoice with the same number and year
/// pass.
#[test]
fn code_00409_credit_note_and_ordinary_invoice() {
    let mut doc = base();
    let ordinary = doc.content.fattura_elettronica_body[0].clone();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .tipo_documento = g::TipoDocumentoType::Td04;
    doc.content.fattura_elettronica_body.push(ordinary);
    assert_absent(&validate(&doc), "00409");
}

/// A document failing several checks at once reports all of them.
#[test]
fn every_violation_is_reported() {
    let mut doc = base();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .numero = "ABC".to_string();
    let l = line(&mut doc);
    l.aliquota_iva = amount("0.50");
    l.natura = Some(g::NaturaType::N1);
    let v = validate(&doc);
    for code in ["00401", "00424", "00425"] {
        assert_has(&v, code);
    }
}

fn sg_id(paese: &str, codice: &str) -> sg::IdFiscaleType {
    sg::IdFiscaleType {
        content: sg::IdFiscaleTypeContent {
            id_paese: paese.to_string(),
            id_codice: codice.to_string(),
        },
    }
}

fn doc_simplified() -> FatturaElettronicaSemplificata {
    FatturaElettronicaSemplificata {
        versione: sg::FormatoTrasmissioneType::Fsm10,
        sistema_emittente: None,
        content: sg::FatturaElettronicaTypeContent {
            fattura_elettronica_header: sg::FatturaElettronicaHeaderType {
                content: sg::FatturaElettronicaHeaderTypeContent {
                    dati_trasmissione: sg::DatiTrasmissioneType {
                        content: sg::DatiTrasmissioneTypeContent {
                            id_trasmittente: sg_id("IT", "01234567890"),
                            progressivo_invio: "00001".to_string(),
                            formato_trasmissione: sg::FormatoTrasmissioneType::Fsm10,
                            codice_destinatario: "ABC1234".to_string(),
                            pec_destinatario: None,
                        },
                    },
                    cedente_prestatore: sg::CedentePrestatoreType {
                        content: sg::CedentePrestatoreTypeContent {
                            id_fiscale_iva: sg_id("IT", "01234567890"),
                            codice_fiscale: None,
                            content_49: sg::CedentePrestatoreContent49Type {
                                content: sg::CedentePrestatoreContent49TypeContent::Content50(
                                    sg::CedentePrestatoreContent50Type {
                                        content: sg::CedentePrestatoreContent50TypeContent {
                                            denominazione: "ALPHA SRL".to_string(),
                                        },
                                    },
                                ),
                            },
                            sede: sg::IndirizzoType {
                                content: sg::IndirizzoTypeContent {
                                    indirizzo: "VIA ROMA 1".to_string(),
                                    numero_civico: None,
                                    cap: "00100".to_string(),
                                    comune: "ROMA".to_string(),
                                    provincia: None,
                                    nazione: "IT".to_string(),
                                },
                            },
                            stabile_organizzazione: None,
                            rappresentante_fiscale: None,
                            iscrizione_rea: None,
                            regime_fiscale: sg::RegimeFiscaleType::Rf01,
                        },
                    },
                    cessionario_committente: sg::CessionarioCommittenteType {
                        content: sg::CessionarioCommittenteTypeContent {
                            identificativi_fiscali: sg::IdentificativiFiscaliType {
                                content: sg::IdentificativiFiscaliTypeContent {
                                    id_fiscale_iva: None,
                                    codice_fiscale: Some("AB123456789".to_string()),
                                },
                            },
                            altri_dati_identificativi: None,
                        },
                    },
                    soggetto_emittente: None,
                },
            },
            fattura_elettronica_body: vec![sg::FatturaElettronicaBodyType {
                content: sg::FatturaElettronicaBodyTypeContent {
                    dati_generali: sg::DatiGeneraliType {
                        content: sg::DatiGeneraliTypeContent {
                            dati_generali_documento: sg::DatiGeneraliDocumentoType {
                                content: sg::DatiGeneraliDocumentoTypeContent {
                                    tipo_documento: sg::TipoDocumentoType::Td07,
                                    divisa: "EUR".to_string(),
                                    data: date(2026, 11, 4),
                                    numero: "42".to_string(),
                                    bollo_virtuale: None,
                                },
                            },
                            dati_fattura_rettificata: None,
                        },
                    },
                    dati_beni_servizi: vec![sg::DatiBeniServiziType {
                        content: sg::DatiBeniServiziTypeContent {
                            descrizione: "CARRELLO DELLA SPESA".to_string(),
                            importo: amount("100.00"),
                            dati_iva: sg::DatiIvaType {
                                content: sg::DatiIvaTypeContent {
                                    imposta: Some(amount("22.00")),
                                    aliquota: Some(amount("22.00")),
                                },
                            },
                            natura: None,
                            riferimento_normativo: None,
                        },
                    }],
                    allegati: Vec::new(),
                },
            }],
            signature: None,
        },
    }
}

/// The base simplified document passes every check the crate implements.
#[test]
fn base_simplified_document_passes() {
    let doc = doc_simplified();
    assert!(
        validate_simplified(&doc).is_empty(),
        "{:?}",
        validate_simplified(&doc)
    );
}

/// 00313: the `XXXXXXX` `CodiceDestinatario` of a simplified invoice to a
/// buyer established in `IT` triggers it, as does the base buyer that
/// carries only its `CodiceFiscale`; a buyer `IdFiscaleIVA` established
/// outside `IT` passes, and so does the base code.
#[test]
fn code_00313_simplified() {
    let mut doc = doc_simplified();
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "XXXXXXX".to_string();
    assert_has(&validate_simplified(&doc), "00313");

    let mut doc = doc_simplified();
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "XXXXXXX".to_string();
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .identificativi_fiscali
        .content
        .id_fiscale_iva = Some(sg_id("IT", "IT0987654321"));
    assert_has(&validate_simplified(&doc), "00313");

    let mut doc = doc_simplified();
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "XXXXXXX".to_string();
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .identificativi_fiscali
        .content
        .id_fiscale_iva = Some(sg_id("DE", "DE0987654321"));
    assert_absent(&validate_simplified(&doc), "00313");

    let doc = doc_simplified();
    assert_absent(&validate_simplified(&doc), "00313");
}

/// 00417: a simplified buyer with neither identifier triggers it; the base
/// buyer with a tax code passes.
#[test]
fn code_00417_simplified() {
    let mut doc = doc_simplified();
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .identificativi_fiscali
        .content
        .codice_fiscale = None;
    assert_has(&validate_simplified(&doc), "00417");

    let doc = doc_simplified();
    assert_absent(&validate_simplified(&doc), "00417");
}

/// 00418: a simplified invoice dated before its `DataFR` triggers it; a
/// `DataFR` on the document date passes.
#[test]
fn code_00418_simplified() {
    let mut doc = doc_simplified();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_fattura_rettificata = Some(sg::DatiFatturaRettificataType {
        content: sg::DatiFatturaRettificataTypeContent {
            numero_fr: "00001A".to_string(),
            data_fr: date(2026, 11, 10),
            elementi_rettificati: "CARRELLO DELLA SPESA".to_string(),
        },
    });
    assert_has(&validate_simplified(&doc), "00418");

    let mut doc = doc_simplified();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_fattura_rettificata = Some(sg::DatiFatturaRettificataType {
        content: sg::DatiFatturaRettificataTypeContent {
            numero_fr: "00001A".to_string(),
            data_fr: date(2026, 11, 4),
            elementi_rettificati: "CARRELLO DELLA SPESA".to_string(),
        },
    });
    assert_absent(&validate_simplified(&doc), "00418");
}

/// 00424: a simplified rate below 1.00 is not a percentage; the base rate
/// passes.
#[test]
fn code_00424_simplified() {
    let mut doc = doc_simplified();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi[0]
        .content
        .dati_iva
        .content
        .aliquota = Some(amount("0.50"));
    assert_has(&validate_simplified(&doc), "00424");

    let doc = doc_simplified();
    assert_absent(&validate_simplified(&doc), "00424");
}

/// 00425: a simplified `Numero` without any digit triggers it; the base
/// number passes.
#[test]
fn code_00425_simplified() {
    let mut doc = doc_simplified();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .numero = "ABC".to_string();
    assert_has(&validate_simplified(&doc), "00425");

    let doc = doc_simplified();
    assert_absent(&validate_simplified(&doc), "00425");
}

/// 00427: a 6 character `CodiceDestinatario` in `FSM10` triggers it; the
/// base 7 character code passes.
#[test]
fn code_00427_simplified() {
    let mut doc = doc_simplified();
    doc.content
        .fattura_elettronica_header
        .content
        .dati_trasmissione
        .content
        .codice_destinatario = "ABC123".to_string();
    assert_has(&validate_simplified(&doc), "00427");

    let doc = doc_simplified();
    assert_absent(&validate_simplified(&doc), "00427");
}

/// 00445: the generic nature `N2` in a simplified `DatiBeniServizi` is not
/// allowed; a detailed nature passes.
#[test]
fn code_00445_simplified() {
    let mut doc = doc_simplified();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi[0]
        .content
        .natura = Some(sg::NaturaType::N2);
    assert_has(&validate_simplified(&doc), "00445");

    let mut doc = doc_simplified();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi[0]
        .content
        .natura = Some(sg::NaturaType::N21);
    assert_absent(&validate_simplified(&doc), "00445");
}

/// 00424: a simplified rate of exactly 1.00 is a percentage.
#[test]
fn code_00424_simplified_rate_of_one_percent() {
    let mut doc = doc_simplified();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_beni_servizi[0]
        .content
        .dati_iva
        .content
        .aliquota = Some(amount("1.00"));
    assert_absent(&validate_simplified(&doc), "00424");
}

/// 00476: a simplified seller and buyer outside `IT` trigger it; a buyer in
/// `IT` passes.
#[test]
fn code_00476_simplified() {
    let mut doc = doc_simplified();
    doc.content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .id_fiscale_iva
        .content
        .id_paese = "US".to_string();
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .identificativi_fiscali
        .content
        .id_fiscale_iva = Some(sg_id("FR", "FR0987654321"));
    assert_has(&validate_simplified(&doc), "00476");

    let mut doc = doc_simplified();
    doc.content
        .fattura_elettronica_header
        .content
        .cedente_prestatore
        .content
        .id_fiscale_iva
        .content
        .id_paese = "US".to_string();
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .identificativi_fiscali
        .content
        .id_fiscale_iva = Some(sg_id("IT", "IT0987654321"));
    assert_absent(&validate_simplified(&doc), "00476");
}

/// 00471: for `TD07` the simplified seller and the buyer must differ. The
/// same parties trigger it, the base buyer with a tax code passes, and the
/// same parties with a `TD08` correction pass.
#[test]
fn code_00471_simplified() {
    let mut doc = doc_simplified();
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .identificativi_fiscali
        .content
        .id_fiscale_iva = Some(sg_id("IT", "01234567890"));
    assert_has(&validate_simplified(&doc), "00471");

    let mut doc = doc_simplified();
    doc.content.fattura_elettronica_body[0]
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content
        .tipo_documento = sg::TipoDocumentoType::Td08;
    doc.content
        .fattura_elettronica_header
        .content
        .cessionario_committente
        .content
        .identificativi_fiscali
        .content
        .id_fiscale_iva = Some(sg_id("IT", "01234567890"));
    assert_absent(&validate_simplified(&doc), "00471");

    let doc = doc_simplified();
    assert_absent(&validate_simplified(&doc), "00471");
}
