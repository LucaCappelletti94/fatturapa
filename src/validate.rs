//! The SDI checks a `FatturaPA` document can decide on its own.
//!
//! The module implements every check of the official *Elenco dei controlli
//! effettuati sul file fattura del Sistema di Interscambio*, version 2.0,
//! that the document alone can decide, and reports each violation with its
//! SDI error code, a short English message and the path of the offending
//! element. `validate` returns every violation it finds, not only the first.
//!
//! The 00471 and 00472 criteria follow the *Allegato A* technical
//! specification 1.9.1, history row 1.7.1 of 01/10/2022, rather than the
//! version 2.0 list, which predates the extension to `TD01`, `TD02`, `TD03`,
//! `TD06`, `TD24`, `TD25` and `TD28` of the ordinary invoice, `TD07` of the
//! simplified invoice and `TD27` for 00472.
//!
//! Paths use the official element names, with 1-based indices for repeated
//! elements, relative to the document root, such as
//! `FatturaElettronicaBody[1]/DatiBeniServizi/DettaglioLinee[2]/Natura`.
//!
//! The checks 00421, 00422 and 00423 keep the tolerances of the Elenco
//! Controlli, one cent for the first and the last, one euro for the
//! middle, and the 00421 rounding rounds down when the third decimal is
//! below 5 and up otherwise.
//!
//! | Code | Meaning | Ordinary invoice | Simplified invoice |
//! |---|---|---|---|
//! | 00001 | invalid file name | out of reach, needs the file name and the SDI state | out of reach |
//! | 00002 | duplicate file name | out of reach, SDI state | out of reach |
//! | 00003 | file size above the channel limit | out of reach, channel context | out of reach |
//! | 00100 | signature certificate expired | out of reach, signature verification | out of reach |
//! | 00101 | signature certificate revoked | out of reach | out of reach |
//! | 00102 | signature not valid | out of reach | out of reach |
//! | 00103 | signature lacks its time reference | out of reach | out of reach |
//! | 00104 | issuing CA not in the trusted list | out of reach | out of reach |
//! | 00105 | time reference after the reception date | out of reach | out of reach |
//! | 00106 | empty or corrupted file | the generated reader rejects it | the generated reader rejects it |
//! | 00107 | signature certificate not valid | out of reach | out of reach |
//! | 00200 | file not conforming to the format | the generated reader rejects it | the generated reader rejects it |
//! | 00201 | more than 50 format errors | out of reach, SDI context | out of reach |
//! | 00300 | seller `IdCodice` not valid | out of reach, needs the tax registry; the XSD pattern is enforced by the reader | out of reach |
//! | 00301 | buyer `IdCodice` not valid | out of reach, tax registry | out of reach |
//! | 00302 | buyer `CodiceFiscale` not valid | out of reach, tax registry | out of reach |
//! | 00303 | fiscal representative or third party `IdCodice` not valid | out of reach, tax registry | out of reach |
//! | 00304 | fiscal representative `CodiceFiscale` not valid | out of reach, tax registry | out of reach |
//! | 00305 | third party `IdCodice` not valid | out of reach, tax registry | out of reach |
//! | 00306 | third party `CodiceFiscale` not valid | out of reach, tax registry | out of reach |
//! | 00311 | `CodiceDestinatario` not valid | out of reach, needs the IPA registry | out of reach |
//! | 00312 | `CodiceDestinatario` not active | out of reach, IPA registry | out of reach |
//! | 00320 | seller VAT number and tax code not coherent | out of reach, tax registry | out of reach |
//! | 00321 | seller tax code not of an IVA group member | out of reach, tax registry | out of reach |
//! | 00322 | seller tax code missing for an IVA group | out of reach, tax registry | out of reach |
//! | 00323 | seller VAT number ceased for more than 5 years | out of reach, tax registry | out of reach |
//! | 00324 | buyer VAT number and tax code not coherent | out of reach, tax registry | out of reach |
//! | 00398 | IPA office not identified for a Centrale code | out of reach, IPA registry | out of reach |
//! | 00399 | buyer missing from the IPA registry | out of reach, IPA registry | out of reach |
//! | 00400 | line `Natura` missing at a zero `AliquotaIVA` | implemented | out of reach, the simplified invoice has no lines |
//! | 00401 | line `Natura` present at a non-zero `AliquotaIVA` | implemented | out of reach |
//! | 00403 | invoice date after the reception date | out of reach, needs the SDI reception date | out of reach |
//! | 00404 | duplicate invoice | out of reach, SDI state | out of reach |
//! | 00409 | duplicate invoice in the same document | implemented | out of reach, the check is defined for the ordinary invoice |
//! | 00411 | `DatiRitenuta` missing for a line with `Ritenuta SI` | implemented | out of reach, no lines |
//! | 00413 | cassa `Natura` missing at a zero `AliquotaIVA` | implemented | out of reach, no cassa blocks |
//! | 00414 | cassa `Natura` present at a non-zero `AliquotaIVA` | implemented | out of reach |
//! | 00415 | `DatiRitenuta` missing for a cassa with `Ritenuta SI` | implemented | out of reach |
//! | 00417 | buyer `IdFiscaleIVA` and `CodiceFiscale` both absent | implemented | implemented |
//! | 00418 | credit note dated before the linked invoice | implemented | out of reach, no `DatiFattureCollegate` |
//! | 00419 | `DatiRiepilogo` missing for a rate value | implemented | out of reach, no `DatiRiepilogo` |
//! | 00420 | `Natura N6` with `EsigibilitaIVA S` | implemented | out of reach, no `EsigibilitaIVA` |
//! | 00421 | `Imposta` not computed per the specification | implemented | out of reach, no `ImponibileImporto` |
//! | 00422 | `ImponibileImporto` not computed per the specification | implemented | out of reach |
//! | 00423 | `PrezzoTotale` not computed per the specification | implemented | out of reach, no lines |
//! | 00424 | rate not expressed as a percentage | implemented | implemented |
//! | 00425 | `Numero` without any digit | implemented | implemented |
//! | 00427 | `CodiceDestinatario` length wrong for the `FormatoTrasmissione` | implemented | implemented |
//! | 00428 | `FormatoTrasmissione` not coherent with `versione` | guaranteed by construction, the two fields are one enumeration | guaranteed by construction |
//! | 00429 | riepilogo `Natura` missing at a zero `AliquotaIVA` | implemented | out of reach |
//! | 00430 | riepilogo `Natura` present at a non-zero `AliquotaIVA` | implemented | out of reach |
//! | 00437 | document `ScontoMaggiorazione` with `Tipo` and neither value | implemented | out of reach, no such element |
//! | 00438 | line `ScontoMaggiorazione` with `Tipo` and neither value | implemented | out of reach |
//! | 00443 | line or cassa rate missing from the `DatiRiepilogo` blocks | implemented | out of reach |
//! | 00444 | line or cassa `Natura` missing from the `DatiRiepilogo` blocks | implemented | out of reach |
//! | 00445 | the generic nature `N2`, `N3` or `N6` | implemented | implemented, `N6` is not a simplified nature |
//! | 00471 | seller and buyer identical for the document type | implemented | implemented, only for `TD07` |
//! | 00472 | seller and buyer must be identical for `TD21` and `TD27` | implemented | out of reach |
//! | 00473 | seller `IdPaese` not allowed for the document type | implemented | out of reach |
//! | 00474 | `TD21` with a zero-rate line | implemented | out of reach |
//! | 00475 | buyer `IdFiscaleIVA` required for the document type | implemented | out of reach |
//! | 00476 | seller and buyer `IdPaese` both different from `IT` | implemented | implemented |
//! | 00477 | invalidated declaration of intent | out of reach, needs the SDI declaration database | out of reach |

use crate::amount::Amount;
use crate::date::Date;
use crate::generated::fpr12::{
    DatiCassaPrevidenzialeType, DatiDocumentiCorrelatiType, DatiRiepilogoType, DatiRitenutaType,
    DettaglioLineeType, DettaglioLineeTypeContent, EsigibilitaIvaType, FatturaElettronicaBodyType,
    FatturaElettronicaHeaderTypeContent, FormatoTrasmissioneType, NaturaType, RitenutaType,
    ScontoMaggiorazioneType, ScontoMaggiorazioneTypeContent, TipoDocumentoType,
    TipoScontoMaggiorazioneType,
};
use crate::generated::fsm10::NaturaType as NaturaSemplificata;
use crate::{FatturaElettronica, FatturaElettronicaSemplificata};
use rust_decimal::{Decimal, RoundingStrategy};

/// One SDI check violation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Violation {
    /// The SDI error code, such as `00400`.
    pub code: String,
    /// The path of the offending element, relative to the document root.
    pub path: String,
    /// A short English description of the violation.
    pub message: String,
}

impl Violation {
    /// Builds a violation from its parts.
    fn new(code: &str, path: String, message: &str) -> Self {
        Self {
            code: code.to_string(),
            path,
            message: message.to_string(),
        }
    }
}

/// The one cent tolerance of the 00421 and 00423 checks.
const CENT: Decimal = Decimal::from_parts(1, 0, 0, false, 2);

/// The one euro tolerance of the 00422 check.
const EURO: Decimal = Decimal::ONE;

/// Rounds `value` to two decimals, down when the third decimal is below 5
/// and up otherwise, the 00421 rounding of the Elenco Controlli. The result
/// always carries exactly two decimals, since the schemas reject amounts
/// without a decimal part.
pub(crate) fn round_half_2(value: Decimal) -> Decimal {
    let mut rounded = value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    rounded.rescale(2);
    rounded
}

/// The `PrezzoTotale` of a line per the 00423 rule of the Elenco Controlli.
/// A discount or markup applies as an amount when one is present, which
/// ignores any percentage of the same block, and otherwise as a cascading
/// percentage on the progressively adjusted unit price. The result is
/// multiplied by the quantity, 1 when the element is absent.
pub(crate) fn line_total(line: &DettaglioLineeTypeContent) -> Amount {
    let mut unitario = line.prezzo_unitario.0;
    for sm in &line.sconto_maggiorazione {
        let ScontoMaggiorazioneTypeContent {
            tipo,
            percentuale,
            importo,
        } = &sm.content;
        if let Some(importo) = importo {
            unitario = match tipo {
                TipoScontoMaggiorazioneType::Sc => unitario - importo.0,
                TipoScontoMaggiorazioneType::Mg => unitario + importo.0,
            };
        } else if let Some(percentuale) = percentuale {
            let factor = percentuale.0 / Decimal::ONE_HUNDRED;
            unitario = match tipo {
                TipoScontoMaggiorazioneType::Sc => unitario * (Decimal::ONE - factor),
                TipoScontoMaggiorazioneType::Mg => unitario * (Decimal::ONE + factor),
            };
        }
    }
    let quantita = line.quantita.as_ref().map_or(Decimal::ONE, |q| q.0);
    Amount(unitario * quantita)
}

/// The generic nature values 00445 no longer allows.
const fn is_generic_natura(natura: Option<&NaturaType>) -> bool {
    matches!(
        natura,
        Some(NaturaType::N2 | NaturaType::N3 | NaturaType::N6)
    )
}

/// The `N6` family of reverse charge natures.
const fn is_n6(natura: &NaturaType) -> bool {
    matches!(
        natura,
        NaturaType::N6
            | NaturaType::N61
            | NaturaType::N62
            | NaturaType::N63
            | NaturaType::N64
            | NaturaType::N65
            | NaturaType::N66
            | NaturaType::N67
            | NaturaType::N68
            | NaturaType::N69
    )
}

/// Runs every document-local SDI check on `doc` and returns all the
/// violations it finds.
#[must_use]
pub fn validate(doc: &FatturaElettronica) -> Vec<Violation> {
    let mut v = Vec::new();
    let header = &doc.content.fattura_elettronica_header.content;
    check_header(header, &mut v);
    for (bi, body) in doc.content.fattura_elettronica_body.iter().enumerate() {
        check_body(bi, body, header, &mut v);
    }
    check_lot(&doc.content.fattura_elettronica_body, &mut v);
    v
}

/// Runs every document-local SDI check on the simplified invoice `doc` and
/// returns all the violations it finds.
#[must_use]
pub fn validate_simplified(doc: &FatturaElettronicaSemplificata) -> Vec<Violation> {
    let mut v = Vec::new();
    let header = &doc.content.fattura_elettronica_header.content;
    let cod = &header.dati_trasmissione.content.codice_destinatario;
    if cod.len() == 6 {
        v.push(Violation::new(
            "00427",
            "FatturaElettronicaHeader/DatiTrasmissione/CodiceDestinatario".to_string(),
            "a 6 character CodiceDestinatario is not allowed in FSM10",
        ));
    }
    let buyer = &header
        .cessionario_committente
        .content
        .identificativi_fiscali
        .content;
    if buyer.id_fiscale_iva.is_none() && buyer.codice_fiscale.is_none() {
        v.push(Violation::new(
            "00417",
            "FatturaElettronicaHeader/CessionarioCommittente/IdentificativiFiscali".to_string(),
            "the buyer has neither IdFiscaleIVA nor CodiceFiscale",
        ));
    }
    let seller = &header.cedente_prestatore.content.id_fiscale_iva;
    if seller.content.id_paese != "IT"
        && buyer
            .id_fiscale_iva
            .as_ref()
            .is_some_and(|buyer| buyer.content.id_paese != "IT")
    {
        v.push(Violation::new(
            "00476",
            "FatturaElettronicaHeader/CedentePrestatore/IdFiscaleIVA/IdPaese".to_string(),
            "the seller and the buyer IdPaese are both different from IT",
        ));
    }
    for (bi, body) in doc.content.fattura_elettronica_body.iter().enumerate() {
        let bpath = format!("FatturaElettronicaBody[{}]", bi + 1);
        let doc_data = &body
            .content
            .dati_generali
            .content
            .dati_generali_documento
            .content;
        if doc_data.tipo_documento == crate::generated::fsm10::TipoDocumentoType::Td07
            && buyer
                .id_fiscale_iva
                .as_ref()
                .is_some_and(|buyer| buyer == seller)
        {
            v.push(Violation::new(
                "00471",
                format!("{bpath}/DatiGenerali/DatiGeneraliDocumento/TipoDocumento"),
                "the seller and the buyer are identical for this TipoDocumento",
            ));
        }
        if !doc_data.numero.bytes().any(|b| b.is_ascii_digit()) {
            v.push(Violation::new(
                "00425",
                format!("{bpath}/DatiGenerali/DatiGeneraliDocumento/Numero"),
                "Numero contains no digit",
            ));
        }
        for (si, beni) in body.content.dati_beni_servizi.iter().enumerate() {
            let spath = format!("{bpath}/DatiBeniServizi[{}]", si + 1);
            let beni = &beni.content;
            let aliquota = beni.dati_iva.content.aliquota.as_ref();
            if aliquota.is_some_and(|a| a.0 != Decimal::ZERO && a.0 < Decimal::ONE) {
                v.push(Violation::new(
                    "00424",
                    format!("{spath}/DatiIVA/Aliquota"),
                    "Aliquota is not expressed as a percentage",
                ));
            }
            if matches!(
                beni.natura,
                Some(NaturaSemplificata::N2 | NaturaSemplificata::N3)
            ) {
                v.push(Violation::new(
                    "00445",
                    format!("{spath}/Natura"),
                    "the generic Natura N2 or N3 is not allowed",
                ));
            }
        }
    }
    v
}

/// The header-level checks 00417, 00427 and 00428.
fn check_header(header: &FatturaElettronicaHeaderTypeContent, v: &mut Vec<Violation>) {
    let trasmissione = &header.dati_trasmissione.content;
    let cod = &trasmissione.codice_destinatario;
    match (&trasmissione.formato_trasmissione, cod.len()) {
        (FormatoTrasmissioneType::Fpa12, 7) | (FormatoTrasmissioneType::Fpr12, 6) => {
            v.push(Violation::new(
                "00427",
                "FatturaElettronicaHeader/DatiTrasmissione/CodiceDestinatario".to_string(),
                "the CodiceDestinatario length is not coherent with the FormatoTrasmissione",
            ));
        }
        _ => {}
    }
    let buyer = &header
        .cessionario_committente
        .content
        .dati_anagrafici
        .content;
    if buyer.id_fiscale_iva.is_none() && buyer.codice_fiscale.is_none() {
        v.push(Violation::new(
            "00417",
            "FatturaElettronicaHeader/CessionarioCommittente/DatiAnagrafici".to_string(),
            "the buyer has neither IdFiscaleIVA nor CodiceFiscale",
        ));
    }
    let seller = &header
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva;
    if seller.content.id_paese != "IT"
        && buyer
            .id_fiscale_iva
            .as_ref()
            .is_some_and(|buyer| buyer.content.id_paese != "IT")
    {
        v.push(Violation::new(
            "00476",
            "FatturaElettronicaHeader/CedentePrestatore/DatiAnagrafici/IdFiscaleIVA/IdPaese"
                .to_string(),
            "the seller and the buyer IdPaese are both different from IT",
        ));
    }
}

/// The body-level checks on one `FatturaElettronicaBody`.
fn check_body(
    bi: usize,
    body: &FatturaElettronicaBodyType,
    header: &FatturaElettronicaHeaderTypeContent,
    v: &mut Vec<Violation>,
) {
    let bpath = format!("FatturaElettronicaBody[{}]", bi + 1);
    let generali = &body.content.dati_generali.content;
    let doc_data = &generali.dati_generali_documento.content;
    let beni = &body.content.dati_beni_servizi.content;
    check_numero(&doc_data.numero, &bpath, v);
    check_lines(&beni.dettaglio_linee, &doc_data.tipo_documento, &bpath, v);
    check_cassa(&doc_data.dati_cassa_previdenziale, &bpath, v);
    check_ritenuta(
        &beni.dettaglio_linee,
        &doc_data.dati_cassa_previdenziale,
        &doc_data.dati_ritenuta,
        &bpath,
        v,
    );
    check_rate_coverage(
        &beni.dettaglio_linee,
        &doc_data.dati_cassa_previdenziale,
        &beni.dati_riepilogo,
        &bpath,
        v,
    );
    check_riepilogo(
        &beni.dati_riepilogo,
        &beni.dettaglio_linee,
        &doc_data.dati_cassa_previdenziale,
        &bpath,
        v,
    );
    check_sconti(&doc_data.sconto_maggiorazione, &bpath, v);
    check_credit_note_dates(
        &generali.dati_fatture_collegate,
        doc_data.data,
        &doc_data.tipo_documento,
        &bpath,
        v,
    );
    check_parties(bi, &doc_data.tipo_documento, header, v);
}

/// The 00425 check on `Numero`.
fn check_numero(numero: &str, bpath: &str, v: &mut Vec<Violation>) {
    if !numero.bytes().any(|b| b.is_ascii_digit()) {
        v.push(Violation::new(
            "00425",
            format!("{bpath}/DatiGenerali/DatiGeneraliDocumento/Numero"),
            "Numero contains no digit",
        ));
    }
}

/// The line checks 00400, 00401, 00423, 00424, 00438, 00445 and 00474.
fn check_lines(
    lines: &[DettaglioLineeType],
    tipo: &TipoDocumentoType,
    bpath: &str,
    v: &mut Vec<Violation>,
) {
    for (li, line) in lines.iter().enumerate() {
        let lpath = format!("{bpath}/DatiBeniServizi/DettaglioLinee[{}]", li + 1);
        let line = &line.content;
        if line.aliquota_iva.0 == Decimal::ZERO && line.natura.is_none() {
            v.push(Violation::new(
                "00400",
                format!("{lpath}/Natura"),
                "Natura is missing on a line whose AliquotaIVA is zero",
            ));
        } else if line.aliquota_iva.0 != Decimal::ZERO && line.natura.is_some() {
            v.push(Violation::new(
                "00401",
                format!("{lpath}/Natura"),
                "Natura is present on a line whose AliquotaIVA is not zero",
            ));
        }
        if (line.prezzo_totale.0 - line_total(line).0).abs() >= CENT {
            v.push(Violation::new(
                "00423",
                format!("{lpath}/PrezzoTotale"),
                "PrezzoTotale does not match the adjusted unit price times the quantity within one cent",
            ));
        }
        if line.aliquota_iva.0 != Decimal::ZERO && line.aliquota_iva.0 < Decimal::ONE {
            v.push(Violation::new(
                "00424",
                format!("{lpath}/AliquotaIVA"),
                "AliquotaIVA is not expressed as a percentage",
            ));
        }
        if is_generic_natura(line.natura.as_ref()) {
            v.push(Violation::new(
                "00445",
                format!("{lpath}/Natura"),
                "the generic Natura N2, N3 or N6 is not allowed",
            ));
        }
        if *tipo == TipoDocumentoType::Td21 && line.aliquota_iva.0 == Decimal::ZERO {
            v.push(Violation::new(
                "00474",
                format!("{lpath}/AliquotaIVA"),
                "a zero AliquotaIVA line is not allowed for this TipoDocumento",
            ));
        }
        for (si, sm) in line.sconto_maggiorazione.iter().enumerate() {
            if sm.content.percentuale.is_none() && sm.content.importo.is_none() {
                v.push(Violation::new(
                    "00438",
                    format!("{lpath}/ScontoMaggiorazione[{}]", si + 1),
                    "ScontoMaggiorazione has a Tipo without Percentuale and Importo",
                ));
            }
        }
    }
}

/// The cassa checks 00413, 00414, 00424 and 00445.
fn check_cassa(cassa: &[DatiCassaPrevidenzialeType], bpath: &str, v: &mut Vec<Violation>) {
    for (ci, c) in cassa.iter().enumerate() {
        let cpath = format!(
            "{bpath}/DatiGenerali/DatiGeneraliDocumento/DatiCassaPrevidenziale[{}]",
            ci + 1
        );
        let c = &c.content;
        if c.aliquota_iva.0 == Decimal::ZERO && c.natura.is_none() {
            v.push(Violation::new(
                "00413",
                format!("{cpath}/Natura"),
                "Natura is missing in a DatiCassaPrevidenziale whose AliquotaIVA is zero",
            ));
        } else if c.aliquota_iva.0 != Decimal::ZERO && c.natura.is_some() {
            v.push(Violation::new(
                "00414",
                format!("{cpath}/Natura"),
                "Natura is present in a DatiCassaPrevidenziale whose AliquotaIVA is not zero",
            ));
        }
        if c.aliquota_iva.0 != Decimal::ZERO && c.aliquota_iva.0 < Decimal::ONE {
            v.push(Violation::new(
                "00424",
                format!("{cpath}/AliquotaIVA"),
                "AliquotaIVA is not expressed as a percentage",
            ));
        }
        if is_generic_natura(c.natura.as_ref()) {
            v.push(Violation::new(
                "00445",
                format!("{cpath}/Natura"),
                "the generic Natura N2, N3 or N6 is not allowed",
            ));
        }
    }
}

/// The 00411 and 00415 checks on `DatiRitenuta`.
fn check_ritenuta(
    lines: &[DettaglioLineeType],
    cassa: &[DatiCassaPrevidenzialeType],
    dati_ritenuta: &[DatiRitenutaType],
    bpath: &str,
    v: &mut Vec<Violation>,
) {
    let path = format!("{bpath}/DatiGenerali/DatiGeneraliDocumento/DatiRitenuta");
    if !dati_ritenuta.is_empty() {
        return;
    }
    if lines
        .iter()
        .any(|l| l.content.ritenuta == Some(RitenutaType::Si))
    {
        v.push(Violation::new(
            "00411",
            path.clone(),
            "DatiRitenuta is missing while a line carries Ritenuta SI",
        ));
    }
    if cassa
        .iter()
        .any(|c| c.content.ritenuta == Some(RitenutaType::Si))
    {
        v.push(Violation::new(
            "00415",
            path,
            "DatiRitenuta is missing while a DatiCassaPrevidenziale carries Ritenuta SI",
        ));
    }
}

/// The 00419, 00443 and 00444 checks: every rate and nature used by a line or
/// a cassa block must appear in the `DatiRiepilogo` blocks.
fn check_rate_coverage(
    lines: &[DettaglioLineeType],
    cassa: &[DatiCassaPrevidenzialeType],
    riep: &[DatiRiepilogoType],
    bpath: &str,
    v: &mut Vec<Violation>,
) {
    let mut rates: Vec<Decimal> = Vec::new();
    for c in cassa {
        if !rates.contains(&c.content.aliquota_iva.0) {
            rates.push(c.content.aliquota_iva.0);
        }
    }
    for l in lines {
        if !rates.contains(&l.content.aliquota_iva.0) {
            rates.push(l.content.aliquota_iva.0);
        }
    }
    if riep.len() < rates.len() {
        v.push(Violation::new(
            "00419",
            format!("{bpath}/DatiBeniServizi/DatiRiepilogo"),
            "a VAT rate has no matching DatiRiepilogo block",
        ));
    }

    let rie_rates: Vec<Decimal> = riep.iter().map(|r| r.content.aliquota_iva.0).collect();
    for (ci, c) in cassa.iter().enumerate() {
        if !rie_rates.contains(&c.content.aliquota_iva.0) {
            v.push(Violation::new(
                "00443",
                format!(
                    "{bpath}/DatiGenerali/DatiGeneraliDocumento/DatiCassaPrevidenziale[{}]/AliquotaIVA",
                    ci + 1
                ),
                "the cassa AliquotaIVA has no matching DatiRiepilogo block",
            ));
        }
    }
    for (li, l) in lines.iter().enumerate() {
        if !rie_rates.contains(&l.content.aliquota_iva.0) {
            v.push(Violation::new(
                "00443",
                format!(
                    "{bpath}/DatiBeniServizi/DettaglioLinee[{}]/AliquotaIVA",
                    li + 1
                ),
                "the line AliquotaIVA has no matching DatiRiepilogo block",
            ));
        }
    }

    let rie_nature: Vec<NaturaType> = riep
        .iter()
        .filter_map(|r| r.content.natura.clone())
        .collect();
    for (ci, c) in cassa.iter().enumerate() {
        if c.content
            .natura
            .as_ref()
            .is_some_and(|natura| !rie_nature.contains(natura))
        {
            v.push(Violation::new(
                "00444",
                format!(
                    "{bpath}/DatiGenerali/DatiGeneraliDocumento/DatiCassaPrevidenziale[{}]/Natura",
                    ci + 1
                ),
                "the cassa Natura has no matching DatiRiepilogo block",
            ));
        }
    }
    for (li, l) in lines.iter().enumerate() {
        if l.content
            .natura
            .as_ref()
            .is_some_and(|natura| !rie_nature.contains(natura))
        {
            v.push(Violation::new(
                "00444",
                format!("{bpath}/DatiBeniServizi/DettaglioLinee[{}]/Natura", li + 1),
                "the line Natura has no matching DatiRiepilogo block",
            ));
        }
    }
}

/// The riepilogo checks 00420, 00421, 00422, 00424, 00429, 00430 and 00445.
fn check_riepilogo(
    riep: &[DatiRiepilogoType],
    lines: &[DettaglioLineeType],
    cassa: &[DatiCassaPrevidenzialeType],
    bpath: &str,
    v: &mut Vec<Violation>,
) {
    for (ri, r) in riep.iter().enumerate() {
        let rpath = format!("{bpath}/DatiBeniServizi/DatiRiepilogo[{}]", ri + 1);
        let r = &r.content;
        if r.aliquota_iva.0 == Decimal::ZERO && r.natura.is_none() {
            v.push(Violation::new(
                "00429",
                format!("{rpath}/Natura"),
                "Natura is missing in a DatiRiepilogo whose AliquotaIVA is zero",
            ));
        } else if r.aliquota_iva.0 != Decimal::ZERO && r.natura.is_some() {
            v.push(Violation::new(
                "00430",
                format!("{rpath}/Natura"),
                "Natura is present in a DatiRiepilogo whose AliquotaIVA is not zero",
            ));
        }
        if r.aliquota_iva.0 != Decimal::ZERO && r.aliquota_iva.0 < Decimal::ONE {
            v.push(Violation::new(
                "00424",
                format!("{rpath}/AliquotaIVA"),
                "AliquotaIVA is not expressed as a percentage",
            ));
        }
        if is_generic_natura(r.natura.as_ref()) {
            v.push(Violation::new(
                "00445",
                format!("{rpath}/Natura"),
                "the generic Natura N2, N3 or N6 is not allowed",
            ));
        }
        if r.esigibilita_iva == Some(EsigibilitaIvaType::S) && r.natura.as_ref().is_some_and(is_n6)
        {
            v.push(Violation::new(
                "00420",
                rpath.clone(),
                "Natura N6 cannot combine with EsigibilitaIVA S (split payment)",
            ));
        }
        let imposta =
            round_half_2(r.aliquota_iva.0 * r.imponibile_importo.0 / Decimal::ONE_HUNDRED);
        if (r.imposta.0 - imposta).abs() >= CENT {
            v.push(Violation::new(
                "00421",
                format!("{rpath}/Imposta"),
                "Imposta does not match the rate applied to the ImponibileImporto within one cent",
            ));
        }
    }

    let mut all_rates: Vec<Decimal> = Vec::new();
    for c in cassa {
        if !all_rates.contains(&c.content.aliquota_iva.0) {
            all_rates.push(c.content.aliquota_iva.0);
        }
    }
    for l in lines {
        if !all_rates.contains(&l.content.aliquota_iva.0) {
            all_rates.push(l.content.aliquota_iva.0);
        }
    }
    for r in riep {
        if !all_rates.contains(&r.content.aliquota_iva.0) {
            all_rates.push(r.content.aliquota_iva.0);
        }
    }
    for aliquota in all_rates {
        let sum_rie: Decimal = riep
            .iter()
            .filter(|r| r.content.aliquota_iva.0 == aliquota)
            .map(|r| r.content.imponibile_importo.0)
            .sum();
        let sum_lines: Decimal = lines
            .iter()
            .filter(|l| l.content.aliquota_iva.0 == aliquota)
            .map(|l| l.content.prezzo_totale.0)
            .sum();
        let sum_cassa: Decimal = cassa
            .iter()
            .filter(|c| c.content.aliquota_iva.0 == aliquota)
            .map(|c| c.content.importo_contributo_cassa.0)
            .sum();
        let sum_arro: Decimal = riep
            .iter()
            .filter(|r| r.content.aliquota_iva.0 == aliquota)
            .filter_map(|r| r.content.arrotondamento)
            .map(|a| a.0)
            .sum();
        if (sum_rie - (sum_lines + sum_cassa + sum_arro)).abs() >= EURO {
            v.push(Violation::new(
                "00422",
                format!("{bpath}/DatiBeniServizi/DatiRiepilogo"),
                "ImponibileImporto does not match the line totals, the cassa contributions and the Arrotondamento within one euro",
            ));
        }
    }
}

/// The 00437 check on the document-level `ScontoMaggiorazione` blocks.
fn check_sconti(sconti: &[ScontoMaggiorazioneType], bpath: &str, v: &mut Vec<Violation>) {
    for (si, sm) in sconti.iter().enumerate() {
        if sm.content.percentuale.is_none() && sm.content.importo.is_none() {
            v.push(Violation::new(
                "00437",
                format!(
                    "{bpath}/DatiGenerali/DatiGeneraliDocumento/ScontoMaggiorazione[{}]",
                    si + 1
                ),
                "ScontoMaggiorazione has a Tipo without Percentuale and Importo",
            ));
        }
    }
}

/// The 00418 check: a `TD04` credit note must not be dated before any of its
/// linked invoices.
fn check_credit_note_dates(
    collegate: &[DatiDocumentiCorrelatiType],
    data: Date,
    tipo: &TipoDocumentoType,
    bpath: &str,
    v: &mut Vec<Violation>,
) {
    if *tipo != TipoDocumentoType::Td04 {
        return;
    }
    for (ci, correlato) in collegate.iter().enumerate() {
        if correlato.content.data.as_ref().is_some_and(|d| data < *d) {
            v.push(Violation::new(
                "00418",
                format!("{bpath}/DatiGenerali/DatiFattureCollegate[{}]/Data", ci + 1),
                "the credit note is dated before the linked invoice",
            ));
        }
    }
}

/// The `TipoDocumento` values for which 00471 rejects an identical seller
/// and buyer, per the Allegato A 1.9.1 list.
const IDENTICAL_PARTIES_REJECTED: &[TipoDocumentoType] = &[
    TipoDocumentoType::Td01,
    TipoDocumentoType::Td02,
    TipoDocumentoType::Td03,
    TipoDocumentoType::Td06,
    TipoDocumentoType::Td16,
    TipoDocumentoType::Td17,
    TipoDocumentoType::Td18,
    TipoDocumentoType::Td19,
    TipoDocumentoType::Td20,
    TipoDocumentoType::Td24,
    TipoDocumentoType::Td25,
    TipoDocumentoType::Td28,
    TipoDocumentoType::Td29,
];

/// The `TipoDocumento` values for which 00472 requires an identical seller
/// and buyer, per the Allegato A 1.9.1 list.
const IDENTICAL_PARTIES_REQUIRED: &[TipoDocumentoType] =
    &[TipoDocumentoType::Td21, TipoDocumentoType::Td27];

/// The 00471, 00472, 00473 and 00475 checks on the parties against the
/// document type.
fn check_parties(
    bi: usize,
    tipo: &TipoDocumentoType,
    header: &FatturaElettronicaHeaderTypeContent,
    v: &mut Vec<Violation>,
) {
    let bpath = format!("FatturaElettronicaBody[{}]", bi + 1);
    let seller = &header
        .cedente_prestatore
        .content
        .dati_anagrafici
        .content
        .id_fiscale_iva;
    let buyer = &header
        .cessionario_committente
        .content
        .dati_anagrafici
        .content;
    if IDENTICAL_PARTIES_REJECTED.contains(tipo)
        && buyer
            .id_fiscale_iva
            .as_ref()
            .is_some_and(|buyer| buyer == seller)
    {
        v.push(Violation::new(
            "00471",
            format!("{bpath}/DatiGenerali/DatiGeneraliDocumento/TipoDocumento"),
            "the seller and the buyer are identical for this TipoDocumento",
        ));
    }
    if IDENTICAL_PARTIES_REQUIRED.contains(tipo)
        && buyer
            .id_fiscale_iva
            .as_ref()
            .is_some_and(|buyer| buyer != seller)
    {
        v.push(Violation::new(
            "00472",
            format!("{bpath}/DatiGenerali/DatiGeneraliDocumento/TipoDocumento"),
            "the seller and the buyer differ for this TipoDocumento",
        ));
    }
    if matches!(
        tipo,
        TipoDocumentoType::Td16
            | TipoDocumentoType::Td17
            | TipoDocumentoType::Td18
            | TipoDocumentoType::Td19
            | TipoDocumentoType::Td20
            | TipoDocumentoType::Td22
            | TipoDocumentoType::Td23
            | TipoDocumentoType::Td28
            | TipoDocumentoType::Td29
    ) && buyer.id_fiscale_iva.is_none()
    {
        v.push(Violation::new(
            "00475",
            format!("{bpath}/DatiGenerali/DatiGeneraliDocumento/TipoDocumento"),
            "the buyer IdFiscaleIVA is required for this TipoDocumento",
        ));
    }
    match *tipo {
        TipoDocumentoType::Td17
        | TipoDocumentoType::Td18
        | TipoDocumentoType::Td19
        | TipoDocumentoType::Td28
            if seller.content.id_paese == "IT" =>
        {
            v.push(Violation::new(
                "00473",
                "FatturaElettronicaHeader/CedentePrestatore/DatiAnagrafici/IdFiscaleIVA/IdPaese"
                    .to_string(),
                "an Italian seller is not allowed for this TipoDocumento",
            ));
        }
        TipoDocumentoType::Td29 if seller.content.id_paese != "IT" => {
            v.push(Violation::new(
                "00473",
                "FatturaElettronicaHeader/CedentePrestatore/DatiAnagrafici/IdFiscaleIVA/IdPaese"
                    .to_string(),
                "a non-Italian seller is not allowed for this TipoDocumento",
            ));
        }
        _ => {}
    }
}

/// The 00409 check, on the invoice numbers of the whole document.
fn check_lot(bodies: &[FatturaElettronicaBodyType], v: &mut Vec<Violation>) {
    for (f, body_f) in bodies.iter().enumerate() {
        let df = &body_f
            .content
            .dati_generali
            .content
            .dati_generali_documento
            .content;
        for (_n, body_n) in bodies.iter().enumerate().skip(f + 1) {
            let dn = &body_n
                .content
                .dati_generali
                .content
                .dati_generali_documento
                .content;
            let same = if df.art_73 == Some(crate::generated::fpr12::Art73Type::Si) {
                df.numero == dn.numero && df.data == dn.data
            } else {
                df.numero == dn.numero && df.data.0.year() == dn.data.0.year()
            };
            if !same {
                continue;
            }
            let both_credit_notes = df.tipo_documento == TipoDocumentoType::Td04
                && dn.tipo_documento == TipoDocumentoType::Td04;
            let one_credit_note = df.tipo_documento == TipoDocumentoType::Td04
                || dn.tipo_documento == TipoDocumentoType::Td04;
            if !one_credit_note || both_credit_notes {
                v.push(Violation::new(
                    "00409",
                    format!(
                        "FatturaElettronicaBody[{}]/DatiGenerali/DatiGeneraliDocumento/Numero",
                        f + 1
                    ),
                    "the invoice number repeats an invoice already in the same document",
                ));
            }
        }
    }
}
