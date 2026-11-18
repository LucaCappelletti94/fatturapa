//! The SDI checks a `FatturaPA` document can decide on its own, and the
//! rules of the specification and of the Agenzia guidance the SDI does not
//! enforce.
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
//! The 00401 and 00430 criteria carry the `TD16` exception of the *Allegato
//! A* 1.9.1 text, which the version 2.0 list does not state, and the 00400,
//! 00401 and 00409 checks apply to the simplified invoice per the *Allegato
//! A* 1.9.1 descriptions, which the version 2.0 list scopes to the ordinary
//! invoice.
//!
//! The check 00313 is not in the Elenco Controlli. The crate implements it
//! per the operative specification *Allegato A*, version 1.9.1, which
//! updated the control for the simplified invoice on 01/01/2022.
//!
//! The check 00330 is not in the Elenco Controlli. The crate implements it
//! per the operative specification *Allegato A*, version 1.9.1, whose
//! `PECDestinatario` paragraph forbids the SDI operational mailboxes of
//! the pattern `sdixx@pec.fatturapa.it`.
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
//! | 00300 | transmitter `IdCodice` not valid | out of reach, needs the tax registry; the XSD pattern is enforced by the reader | out of reach |
//! | 00301 | seller `IdCodice` not valid | out of reach, tax registry | out of reach |
//! | 00302 | seller `CodiceFiscale` not valid | out of reach, tax registry | out of reach |
//! | 00303 | seller or buyer fiscal representative `IdCodice` not valid | out of reach, tax registry | out of reach |
//! | 00304 | seller fiscal representative `CodiceFiscale` not valid | out of reach, tax registry | out of reach |
//! | 00305 | buyer `IdCodice` not valid | out of reach, tax registry | out of reach |
//! | 00306 | buyer `CodiceFiscale` not valid | out of reach, tax registry | out of reach |
//! | 00311 | `CodiceDestinatario` not valid | out of reach, needs the IPA registry | out of reach |
//! | 00312 | `CodiceDestinatario` not active | out of reach, IPA registry | out of reach |
//! | 00313 | the `XXXXXXX` `CodiceDestinatario` for a buyer established in `IT` | implemented | implemented |
//! | 00320 | seller VAT number and tax code not coherent | out of reach, tax registry | out of reach |
//! | 00321 | seller tax code not of an IVA group member | out of reach, tax registry | out of reach |
//! | 00322 | seller tax code missing for an IVA group | out of reach, tax registry | out of reach |
//! | 00323 | seller VAT number ceased for more than 5 years | out of reach, tax registry | out of reach |
//! | 00324 | buyer VAT number and tax code not coherent | out of reach, tax registry | out of reach |
//! | 00325 | buyer tax code not of an IVA group member | out of reach, IVA group registry | out of reach |
//! | 00326 | buyer tax code missing for a buyer IVA group | out of reach, IVA group registry | out of reach |
//! | 00327 | buyer group tax code without the group VAT number | out of reach, IVA group registry | out of reach |
//! | 00330 | `PECDestinatario` is an SDI operational PEC mailbox | implemented | implemented |
//! | 00398 | a `Centrale` office code although IPA identifies the specific office | out of reach, IPA registry | out of reach |
//! | 00399 | `FPR12` to a buyer registered in IPA | out of reach, IPA registry | out of reach |
//! | 00400 | the `Natura` missing at a zero rate | implemented | implemented, the `DatiBeniServizi` `Natura` |
//! | 00401 | the `Natura` present at a non-zero rate, the `TD16` is exempt | implemented | implemented, the `DatiBeniServizi` `Natura` |
//! | 00403 | invoice date after the reception date | out of reach, needs the SDI reception date | out of reach |
//! | 00404 | duplicate invoice | out of reach, SDI state | out of reach |
//! | 00409 | duplicate invoice in the same document | implemented | implemented |
//! | 00411 | `DatiRitenuta` missing for a line with `Ritenuta SI` | implemented | out of reach, no lines |
//! | 00413 | cassa `Natura` missing at a zero `AliquotaIVA` | implemented | out of reach, no cassa blocks |
//! | 00414 | cassa `Natura` present at a non-zero `AliquotaIVA` | implemented | out of reach |
//! | 00415 | `DatiRitenuta` missing for a cassa with `Ritenuta SI` | implemented | out of reach |
//! | 00417 | buyer `IdFiscaleIVA` and `CodiceFiscale` both absent | implemented | implemented |
//! | 00418 | linked invoice dated after the document | implemented, only for the `TD04` | implemented, only for the `TD08`, the `Data` before the `DataFR` |
//! | 00419 | `DatiRiepilogo` missing for a rate value | implemented | out of reach, no `DatiRiepilogo` |
//! | 00420 | `Natura N6` with `EsigibilitaIVA S` | implemented | out of reach, no `EsigibilitaIVA` |
//! | 00421 | `Imposta` not computed per the specification | implemented | out of reach, no `ImponibileImporto` |
//! | 00422 | `ImponibileImporto` not computed per the specification | implemented | out of reach |
//! | 00423 | `PrezzoTotale` not computed per the specification | implemented | out of reach, no lines |
//! | 00424 | rate not expressed as a percentage | implemented | implemented |
//! | 00425 | `Numero` without any digit | implemented | implemented |
//! | 00427 | `CodiceDestinatario` length wrong for the `FormatoTrasmissione` | implemented | out of reach, the `FSM10` schema enforces the 7 characters |
//! | 00428 | `FormatoTrasmissione` not coherent with `versione` | guaranteed by construction, the two fields are one enumeration | guaranteed by construction |
//! | 00429 | riepilogo `Natura` missing at a zero `AliquotaIVA` | implemented | out of reach |
//! | 00430 | riepilogo `Natura` present at a non-zero `AliquotaIVA`, the `TD16` is exempt | implemented | out of reach |
//! | 00437 | document `ScontoMaggiorazione` with `Tipo` and neither value | implemented | out of reach, no such element |
//! | 00438 | line `ScontoMaggiorazione` with `Tipo` and neither value | implemented | out of reach |
//! | 00443 | line or cassa rate missing from the `DatiRiepilogo` blocks | implemented | out of reach |
//! | 00444 | line or cassa `Natura` missing from the `DatiRiepilogo` blocks | implemented | out of reach |
//! | 00445 | the generic nature `N2`, `N3` or `N6`, from the invoices dated 1 January 2021 | implemented | implemented, `N6` is not a simplified nature |
//! | 00460 | total amount above the 400.00 euro limit for the simplified invoice | out of reach, the check is defined for the simplified invoice | implemented |
//! | 00471 | seller and buyer identical for the document type | implemented | implemented, only for `TD07` |
//! | 00472 | seller and buyer must be identical for `TD21` and `TD27` | implemented | out of reach |
//! | 00473 | seller `IdPaese` not allowed for the document type | implemented | out of reach |
//! | 00474 | `TD21` with a zero-rate line | implemented | out of reach |
//! | 00475 | buyer `IdFiscaleIVA` required for the document type | implemented | out of reach |
//! | 00476 | seller and buyer `IdPaese` both different from `IT` | implemented | implemented |
//! | 00477 | invalidated declaration of intent | out of reach, needs the SDI declaration database | out of reach |
//!
//! The rules below are not SDI checks, so a document breaking them reaches
//! the buyer, and each one carries a code of its own outside the numeric
//! SDI namespace.
//!
//! | Code | Meaning | Ordinary invoice | Simplified invoice | Source |
//! |---|---|---|---|---|
//! | `RIFERIMENTO-NORMATIVO` | a `Natura` without its `RiferimentoNormativo` | implemented, on the `DatiRiepilogo` | implemented, on the `DatiBeniServizi` | *Allegato A* 1.9.1, the `RiferimentoNormativo` is mandatory for the operations of the `Natura` element |
//! | `CAP-ESTERO` | a seat or permanent establishment abroad with a `CAP` other than `00000` | implemented | implemented | the Agenzia [FAQ on the compilation of the electronic invoice](https://www.agenziaentrate.gov.it/portale/schede/comunicazioni/fatture-e-corrispettivi/faq-fe/risposte-alle-domande-piu-frequenti-categoria/compilazione-della-fattura-elettronica), the generic `00000` for the address abroad |
//! | `PROVINCIA-ESTERA` | a seat or permanent establishment abroad with a `Provincia` | implemented | implemented | *Allegato A* 1.9.1, the `Provincia` is written only for a seat in Italy |
//! | `TRASMITTENTE-CF` | an Italian `IdTrasmittente` carrying the `partita IVA` of a party whose `CodiceFiscale` differs | implemented | implemented | *Allegato A* 1.9.1, the `IdCodice` of a transmitter established in Italy is its codice fiscale |
//! | `BOLLO` | more than 77.47 euros of operations subject to the stamp duty without the `DatiBollo` | implemented | implemented, on the `BolloVirtuale` | the Agenzia guide [*L'imposta di bollo sulle fatture elettroniche*](https://www.agenziaentrate.gov.it/portale/documents/d/guest/l-imposta_di_bollo_sulle_fatture_elettronichegiugno2026), June 2026 |
//!
//! The `BOLLO` rule follows the list B of the guide. It sums the
//! `PrezzoTotale` of the ordinary lines, or the `Importo` of the simplified
//! `DatiBeniServizi`, whose `Natura` is `N2.1`, `N2.2`, `N3.5`, `N3.6` or
//! `N4` and whose `AltriDatiGestionali` carry no `NB1`, `NB2` or `NB3`
//! `TipoDato`, and it skips the `TD16`, `TD17`, `TD18`, `TD19` and `TD28`
//! documents and the `RF05` to `RF11` regimes. The `XXXXXXX` documents stay
//! in, since the guide leaves them out of the list B while the stamp duty
//! on the invoice stays due.

use crate::amount::Amount;
use crate::date::Date;
use crate::generated::fpr12::{
    DatiCassaPrevidenzialeType, DatiDocumentiCorrelatiType, DatiRiepilogoType, DatiRitenutaType,
    DettaglioLineeType, DettaglioLineeTypeContent, EsigibilitaIvaType, FatturaElettronicaBodyType,
    FatturaElettronicaHeaderTypeContent, FormatoTrasmissioneType, NaturaType, RegimeFiscaleType,
    RitenutaType, ScontoMaggiorazioneType, ScontoMaggiorazioneTypeContent, TipoDocumentoType,
    TipoScontoMaggiorazioneType,
};
use crate::generated::fsm10::{
    DatiBeniServiziType as DatiBeniServiziSemplificati,
    FatturaElettronicaBodyType as BodySemplificato,
    FatturaElettronicaHeaderTypeContent as HeaderSemplificato, NaturaType as NaturaSemplificata,
    RegimeFiscaleType as RegimeFiscaleSemplificata, TipoDocumentoType as TipoDocumentoSemplificato,
};
use crate::ids::BolloExemption;
use crate::{FatturaElettronica, FatturaElettronicaSemplificata};
use rust_decimal::{Decimal, RoundingStrategy};

/// One SDI check violation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Violation {
    /// The SDI error code, such as `00400`, or the code of a rule the SDI
    /// does not enforce, such as `BOLLO`.
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

/// The 400.00 euro cap on the total amount of a simplified invoice, check
/// 00460, the art. 21 bis DPR 633/72 limit raised by the DM MEF 10/5/2019.
const SIMPLIFIED_INVOICE_CAP: Decimal = Decimal::from_parts(40000, 0, 0, false, 2);

/// Rounds `value` to two decimals, down when the third decimal is below 5
/// and up otherwise, the 00421 rounding of the Elenco Controlli. The result
/// always carries exactly two decimals, since the schemas reject amounts
/// without a decimal part.
pub(crate) fn round_half_2(value: Decimal) -> Decimal {
    let mut rounded = value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    rounded.rescale(2);
    rounded
}

/// The 00423 `PrezzoTotale` of a line, the unit price adjusted by each discount or markup amount, or else its cascading percentage, times the quantity, `None` on decimal overflow.
pub(crate) fn line_total(line: &DettaglioLineeTypeContent) -> Option<Amount> {
    let mut unitario = line.prezzo_unitario.0;
    for sm in &line.sconto_maggiorazione {
        let ScontoMaggiorazioneTypeContent {
            tipo,
            percentuale,
            importo,
        } = &sm.content;
        if let Some(importo) = importo {
            unitario = match tipo {
                TipoScontoMaggiorazioneType::Sc => unitario.checked_sub(importo.0)?,
                TipoScontoMaggiorazioneType::Mg => unitario.checked_add(importo.0)?,
            };
        } else if let Some(percentuale) = percentuale {
            let factor = percentuale.0 / Decimal::ONE_HUNDRED;
            unitario = match tipo {
                TipoScontoMaggiorazioneType::Sc => unitario.checked_mul(Decimal::ONE - factor)?,
                TipoScontoMaggiorazioneType::Mg => unitario.checked_mul(Decimal::ONE + factor)?,
            };
        }
    }
    let quantita = line.quantita.as_ref().map_or(Decimal::ONE, |q| q.0);
    unitario.checked_mul(quantita).map(Amount)
}

/// The first invoice date 00445 applies to.
const GENERIC_NATURA_RETIRED: jiff::civil::Date = jiff::civil::date(2021, 1, 1);

/// Whether `natura` is a generic nature value 00445 no longer allows on an
/// invoice dated `data`.
fn is_generic_natura(natura: Option<&NaturaType>, data: Date) -> bool {
    data.0 >= GENERIC_NATURA_RETIRED
        && matches!(
            natura,
            Some(NaturaType::N2 | NaturaType::N3 | NaturaType::N6)
        )
}

/// Whether the `PECDestinatario` `pec` is one of the SDI operational
/// mailboxes, check 00330. The *Allegato A* 1.9.1 `PECDestinatario`
/// paragraph forbids the addresses of the pattern
/// `sdixx@pec.fatturapa.it`, the load balanced set of mailboxes the SDI
/// uses to receive the files, the first being `sdi01`. The check is the
/// exact match of the 22 character pattern, the `xx` any two characters,
/// ignoring case, since the e-mail domain is case insensitive per RFC 5321
/// and the `pec.fatturapa.it` domain belongs to the SDI, so no private
/// mailbox is misreported.
fn is_sdi_pec_mailbox(pec: &str) -> bool {
    let bytes = pec.as_bytes();
    bytes.len() == 22
        && bytes[..3].eq_ignore_ascii_case(b"sdi")
        && bytes[5] == b'@'
        && bytes[6..].eq_ignore_ascii_case(b"pec.fatturapa.it")
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

/// Runs every document-local SDI check and every guidance rule on `doc`
/// and returns all the violations it finds.
#[must_use]
pub fn validate(doc: &FatturaElettronica) -> Vec<Violation> {
    let mut v = Vec::new();
    let header = &doc.content.fattura_elettronica_header.content;
    check_header(header, &mut v);
    check_guidance_header(header, &mut v);
    for (bi, body) in doc.content.fattura_elettronica_body.iter().enumerate() {
        check_body(bi, body, header, &mut v);
    }
    check_lot(&doc.content.fattura_elettronica_body, &mut v);
    v
}

/// Runs every document-local SDI check and every guidance rule on the
/// simplified invoice `doc` and returns all the violations it finds.
#[must_use]
pub fn validate_simplified(doc: &FatturaElettronicaSemplificata) -> Vec<Violation> {
    let mut v = Vec::new();
    let header = &doc.content.fattura_elettronica_header.content;
    check_simplified_header(header, &mut v);
    check_simplified_guidance_header(header, &mut v);
    for (bi, body) in doc.content.fattura_elettronica_body.iter().enumerate() {
        check_simplified_body(bi, body, header, &mut v);
        check_simplified_guidance_body(bi, body, header, &mut v);
    }
    check_simplified_lot(&doc.content.fattura_elettronica_body, &mut v);
    v
}

/// The header-level checks of the simplified invoice, 00313, 00330, 00417
/// and 00476.
fn check_simplified_header(header: &HeaderSemplificato, v: &mut Vec<Violation>) {
    if header
        .dati_trasmissione
        .content
        .pec_destinatario
        .as_deref()
        .is_some_and(is_sdi_pec_mailbox)
    {
        v.push(Violation::new(
            "00330",
            "FatturaElettronicaHeader/DatiTrasmissione/PECDestinatario".to_string(),
            "the PECDestinatario is one of the SDI operational PEC mailboxes",
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
    if header.dati_trasmissione.content.codice_destinatario
        == crate::ids::CodiceDestinatario::FOREIGN
        && buyer
            .id_fiscale_iva
            .as_ref()
            .is_none_or(|iva| iva.content.id_paese == "IT")
    {
        v.push(Violation::new(
            "00313",
            "FatturaElettronicaHeader/DatiTrasmissione/CodiceDestinatario".to_string(),
            "the XXXXXXX CodiceDestinatario requires a buyer IdFiscaleIVA with an IdPaese other than IT",
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
}

/// The body-level checks of the simplified invoice, 00400, 00401, 00418,
/// 00424, 00425, 00445, 00460 and 00471. The 00418 applies to the `TD08`
/// only, per the simplified field table 2.1.1.3, and the `DataFR` row
/// 2.1.2.2 states the same comparison from the other side.
fn check_simplified_body(
    bi: usize,
    body: &BodySemplificato,
    header: &HeaderSemplificato,
    v: &mut Vec<Violation>,
) {
    let buyer = &header
        .cessionario_committente
        .content
        .identificativi_fiscali
        .content;
    let seller = &header.cedente_prestatore.content.id_fiscale_iva;
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
    let generali = &body.content.dati_generali.content;
    if doc_data.tipo_documento == crate::generated::fsm10::TipoDocumentoType::Td08
        && generali
            .dati_fattura_rettificata
            .as_ref()
            .is_some_and(|rettificata| doc_data.data < rettificata.content.data_fr)
    {
        v.push(Violation::new(
            "00418",
            format!("{bpath}/DatiGenerali/DatiFatturaRettificata/DataFR"),
            "the linked invoice is dated after the document",
        ));
    }
    check_simplified_cap(body, header, &bpath, v);
    for (si, beni) in body.content.dati_beni_servizi.iter().enumerate() {
        let spath = format!("{bpath}/DatiBeniServizi[{}]", si + 1);
        let beni = &beni.content;
        let aliquota = beni.dati_iva.content.aliquota.as_ref();
        if aliquota.is_some_and(|a| a.0 == Decimal::ZERO) && beni.natura.is_none() {
            v.push(Violation::new(
                "00400",
                format!("{spath}/Natura"),
                "Natura is missing on a DatiBeniServizi whose Aliquota is zero",
            ));
        } else if aliquota.is_some_and(|a| a.0 != Decimal::ZERO) && beni.natura.is_some() {
            v.push(Violation::new(
                "00401",
                format!("{spath}/Natura"),
                "Natura is present on a DatiBeniServizi whose Aliquota is not zero",
            ));
        }
        if aliquota.is_some_and(|a| a.0 != Decimal::ZERO && a.0 < Decimal::ONE) {
            v.push(Violation::new(
                "00424",
                format!("{spath}/DatiIVA/Aliquota"),
                "Aliquota is not expressed as a percentage",
            ));
        }
        if doc_data.data.0 >= GENERIC_NATURA_RETIRED
            && matches!(
                beni.natura,
                Some(NaturaSemplificata::N2 | NaturaSemplificata::N3)
            )
        {
            v.push(Violation::new(
                "00445",
                format!("{spath}/Natura"),
                "the generic Natura N2 or N3 is not allowed",
            ));
        }
    }
}

/// The 00460 check on the total amount of a simplified `FatturaElettronicaBody`.
///
/// The total is the sum of the body `DatiBeniServizi` `Importo`, and it must
/// not exceed the 400.00 euros of art. 21 bis DPR 633/72, the limit raised from
/// 100 by the DM MEF 10/5/2019 and restated by the Allegato A 1.9.1 codice
/// 00460. It is per body, the field 2.2.2 `<Importo>` of the Rappresentazione
/// tabellare of the simplified invoice, summed over the `DatiBeniServizi` when
/// the element is present more than once. It is waived when the body carries a
/// `DatiFatturaRettificata`, the correction of an already-issued invoice, or
/// when the seller is in the forfettario regime `RF19` or the transfrontaliero
/// franchigia IVA regime `RF20`.
fn check_simplified_cap(
    body: &BodySemplificato,
    header: &HeaderSemplificato,
    bpath: &str,
    v: &mut Vec<Violation>,
) {
    let regime_exempt = matches!(
        &header.cedente_prestatore.content.regime_fiscale,
        RegimeFiscaleSemplificata::Rf19 | RegimeFiscaleSemplificata::Rf20
    );
    let totale: Decimal = body
        .content
        .dati_beni_servizi
        .iter()
        .map(|beni| beni.content.importo.0)
        .sum();
    if totale > SIMPLIFIED_INVOICE_CAP
        && body
            .content
            .dati_generali
            .content
            .dati_fattura_rettificata
            .is_none()
        && !regime_exempt
    {
        v.push(Violation::new(
            "00460",
            format!("{bpath}/DatiBeniServizi"),
            "the total amount exceeds the 400 euro limit for the simplified invoice",
        ));
    }
}

/// The header-level checks 00313, 00330, 00417, 00427 and 00476.
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
    if trasmissione
        .pec_destinatario
        .as_deref()
        .is_some_and(is_sdi_pec_mailbox)
    {
        v.push(Violation::new(
            "00330",
            "FatturaElettronicaHeader/DatiTrasmissione/PECDestinatario".to_string(),
            "the PECDestinatario is one of the SDI operational PEC mailboxes",
        ));
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
    if cod == crate::ids::CodiceDestinatario::FOREIGN
        && buyer
            .id_fiscale_iva
            .as_ref()
            .is_none_or(|iva| iva.content.id_paese == "IT")
    {
        v.push(Violation::new(
            "00313",
            "FatturaElettronicaHeader/DatiTrasmissione/CodiceDestinatario".to_string(),
            "the XXXXXXX CodiceDestinatario requires a buyer IdFiscaleIVA with an IdPaese other than IT",
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
    check_lines(
        &beni.dettaglio_linee,
        &doc_data.tipo_documento,
        doc_data.data,
        &bpath,
        v,
    );
    check_cassa(&doc_data.dati_cassa_previdenziale, doc_data.data, &bpath, v);
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
        &doc_data.tipo_documento,
        doc_data.data,
        &bpath,
        v,
    );
    check_sconti(&doc_data.sconto_maggiorazione, &bpath, v);
    check_linked_invoice_dates(
        &generali.dati_fatture_collegate,
        &doc_data.tipo_documento,
        doc_data.data,
        &bpath,
        v,
    );
    check_parties(bi, &doc_data.tipo_documento, header, v);
    check_guidance_body(body, header, &bpath, v);
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
/// 00401 carries the `TD16` exception of the Allegato A 1.9.1 text, which
/// allows a line `Natura` at a non-zero `AliquotaIVA` on the integration of
/// the reverse charge invoice.
fn check_lines(
    lines: &[DettaglioLineeType],
    tipo: &TipoDocumentoType,
    data: Date,
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
        } else if line.aliquota_iva.0 != Decimal::ZERO
            && line.natura.is_some()
            && *tipo != TipoDocumentoType::Td16
        {
            v.push(Violation::new(
                "00401",
                format!("{lpath}/Natura"),
                "Natura is present on a line whose AliquotaIVA is not zero",
            ));
        }
        if line_total(line)
            .and_then(|total| line.prezzo_totale.0.checked_sub(total.0))
            .is_none_or(|difference| difference.abs() > CENT)
        {
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
        if is_generic_natura(line.natura.as_ref(), data) {
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
fn check_cassa(
    cassa: &[DatiCassaPrevidenzialeType],
    data: Date,
    bpath: &str,
    v: &mut Vec<Violation>,
) {
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
        if is_generic_natura(c.natura.as_ref(), data) {
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
/// 00430 carries the `TD16` exception of the Allegato A 1.9.1 text, which
/// allows a riepilogo `Natura` at a non-zero `AliquotaIVA` on the
/// integration of the reverse charge invoice.
fn check_riepilogo(
    riep: &[DatiRiepilogoType],
    lines: &[DettaglioLineeType],
    cassa: &[DatiCassaPrevidenzialeType],
    tipo: &TipoDocumentoType,
    data: Date,
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
        } else if r.aliquota_iva.0 != Decimal::ZERO
            && r.natura.is_some()
            && *tipo != TipoDocumentoType::Td16
        {
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
        if is_generic_natura(r.natura.as_ref(), data) {
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
        if (r.imposta.0 - imposta).abs() > CENT {
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
        if (sum_rie - (sum_lines + sum_cassa + sum_arro)).abs() > EURO {
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

/// The 00418 check: the document `Data` of a `TD04` must not be before the
/// `Data` of any linked invoice. The ordinary field table, row 2.1.1.3
/// `<Data>`, scopes the check to the `TD04`, and the Elenco Controlli 2.0
/// states `IF ( TipoDoc == "TD04" ) IF ( DataNC < DataFattColl ) ERRORE
/// (00418)`. The *Allegato A* 1.9.1 prose on the `DatiFattureCollegate`
/// `<Data>` reads wider, for every `TipoDocumento`.
fn check_linked_invoice_dates(
    collegate: &[DatiDocumentiCorrelatiType],
    tipo: &TipoDocumentoType,
    data: Date,
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
                "the linked invoice is dated after the document",
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
            let art_73 = df.art_73 == Some(crate::generated::fpr12::Art73Type::Si)
                && dn.art_73 == Some(crate::generated::fpr12::Art73Type::Si);
            let same = if art_73 {
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

/// The 00409 check, on the invoice numbers of the whole simplified
/// document. The `TD08` correction of an already-issued invoice waives the
/// check for a pair of which it is one member, and the simplified invoice
/// carries no `Art73`, so the comparison is always on the year.
fn check_simplified_lot(bodies: &[BodySemplificato], v: &mut Vec<Violation>) {
    for (f, body_f) in bodies.iter().enumerate() {
        let df = &body_f
            .content
            .dati_generali
            .content
            .dati_generali_documento
            .content;
        for body_n in bodies.iter().skip(f + 1) {
            let dn = &body_n
                .content
                .dati_generali
                .content
                .dati_generali_documento
                .content;
            if df.numero != dn.numero || df.data.0.year() != dn.data.0.year() {
                continue;
            }
            let both_credit_notes = df.tipo_documento == TipoDocumentoSemplificato::Td08
                && dn.tipo_documento == TipoDocumentoSemplificato::Td08;
            let one_credit_note = df.tipo_documento == TipoDocumentoSemplificato::Td08
                || dn.tipo_documento == TipoDocumentoSemplificato::Td08;
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

/// The amount the operations subject to the stamp duty must exceed for
/// the 2.00 euros duty to fall due, art. 13 of the tariff of DPR 642/1972.
pub(crate) const BOLLO_THRESHOLD: Decimal = Decimal::from_parts(7747, 0, 0, false, 2);

/// The CAP the Agenzia prescribes for an address abroad.
const FOREIGN_CAP: &str = "00000";

/// Whether `natura` is one the Agenzia counts towards the stamp duty, the
/// operations not subject to VAT, `N2.1` and `N2.2`, not taxable, `N3.5`
/// and `N3.6`, and exempt, `N4`.
const fn counts_for_bollo(natura: &NaturaType) -> bool {
    matches!(
        natura,
        NaturaType::N21 | NaturaType::N22 | NaturaType::N35 | NaturaType::N36 | NaturaType::N4
    )
}

/// Whether the line opts out of the stamp duty with an `NB1`, `NB2` or
/// `NB3` `TipoDato`.
fn exempt_from_bollo(line: &DettaglioLineeTypeContent) -> bool {
    line.altri_dati_gestionali.iter().any(|dato| {
        BolloExemption::ALL
            .iter()
            .any(|exemption| dato.content.tipo_dato == exemption.code())
    })
}

/// Whether the ordinary invoice of `tipo`, issued under `regime`, owes the
/// stamp duty on `lines`, per the list B of the Agenzia guide of June 2026.
pub(crate) fn bollo_due<'a>(
    tipo: &TipoDocumentoType,
    regime: &RegimeFiscaleType,
    lines: impl IntoIterator<Item = &'a DettaglioLineeTypeContent>,
) -> bool {
    let excluded_tipo = matches!(
        tipo,
        TipoDocumentoType::Td16
            | TipoDocumentoType::Td17
            | TipoDocumentoType::Td18
            | TipoDocumentoType::Td19
            | TipoDocumentoType::Td28
    );
    let excluded_regime = matches!(
        regime,
        RegimeFiscaleType::Rf05
            | RegimeFiscaleType::Rf06
            | RegimeFiscaleType::Rf07
            | RegimeFiscaleType::Rf08
            | RegimeFiscaleType::Rf09
            | RegimeFiscaleType::Rf10
            | RegimeFiscaleType::Rf11
    );
    if excluded_tipo || excluded_regime {
        return false;
    }
    let subject: Decimal = lines
        .into_iter()
        .filter(|line| {
            line.natura.as_ref().is_some_and(counts_for_bollo) && !exempt_from_bollo(line)
        })
        .map(|line| line.prezzo_totale.0)
        .sum();
    subject > BOLLO_THRESHOLD
}

/// Whether the simplified invoice issued under `regime` owes the stamp
/// duty on `beni`, per the list B of the Agenzia guide of June 2026, which
/// excludes none of the simplified document types.
fn simplified_bollo_due(
    regime: &RegimeFiscaleSemplificata,
    beni: &[DatiBeniServiziSemplificati],
) -> bool {
    let excluded_regime = matches!(
        regime,
        RegimeFiscaleSemplificata::Rf05
            | RegimeFiscaleSemplificata::Rf06
            | RegimeFiscaleSemplificata::Rf07
            | RegimeFiscaleSemplificata::Rf08
            | RegimeFiscaleSemplificata::Rf09
            | RegimeFiscaleSemplificata::Rf10
            | RegimeFiscaleSemplificata::Rf11
    );
    let subject: Decimal = beni
        .iter()
        .map(|beni| &beni.content)
        .filter(|beni| {
            matches!(
                beni.natura,
                Some(
                    NaturaSemplificata::N21
                        | NaturaSemplificata::N22
                        | NaturaSemplificata::N35
                        | NaturaSemplificata::N36
                        | NaturaSemplificata::N4
                )
            )
        })
        .map(|beni| beni.importo.0)
        .sum();
    !excluded_regime && subject > BOLLO_THRESHOLD
}

/// The `CAP-ESTERO` and `PROVINCIA-ESTERA` rules on the address at `path`.
fn check_foreign_seat(
    path: &str,
    nazione: &str,
    cap: &str,
    provincia: Option<&str>,
    v: &mut Vec<Violation>,
) {
    if nazione == "IT" {
        return;
    }
    if cap != FOREIGN_CAP {
        v.push(Violation::new(
            "CAP-ESTERO",
            format!("{path}/CAP"),
            "an address abroad carries a CAP other than the generic 00000",
        ));
    }
    if provincia.is_some() {
        v.push(Violation::new(
            "PROVINCIA-ESTERA",
            format!("{path}/Provincia"),
            "an address abroad carries a Provincia",
        ));
    }
}

/// The fiscal identifiers of a party, its `partita IVA` as the `IdPaese`
/// and `IdCodice` pair and its codice fiscale.
struct FiscalIds<'a> {
    /// The `IdFiscaleIVA`, as `IdPaese` and `IdCodice`.
    partita_iva: Option<(&'a str, &'a str)>,
    /// The `CodiceFiscale`.
    codice_fiscale: Option<&'a str>,
}

impl FiscalIds<'_> {
    /// Whether the Italian transmitter `(paese, codice)` carries the
    /// `partita IVA` of the party while the party codice fiscale differs.
    fn lend_partita_iva_to(&self, (paese, codice): (&str, &str)) -> bool {
        paese == "IT"
            && self.partita_iva == Some(("IT", codice))
            && self
                .codice_fiscale
                .is_some_and(|codice_fiscale| codice_fiscale != codice)
    }
}

/// The `TRASMITTENTE-CF` rule on the transmitter and the two parties.
fn check_trasmittente(
    trasmittente: (&str, &str),
    parties: &[FiscalIds<'_>],
    v: &mut Vec<Violation>,
) {
    if parties
        .iter()
        .any(|party| party.lend_partita_iva_to(trasmittente))
    {
        v.push(Violation::new(
            "TRASMITTENTE-CF",
            "FatturaElettronicaHeader/DatiTrasmissione/IdTrasmittente/IdCodice".to_string(),
            "the transmitter is identified by the partita IVA of a party whose CodiceFiscale differs",
        ));
    }
}

/// The `CAP-ESTERO`, `PROVINCIA-ESTERA` and `TRASMITTENTE-CF` rules on the
/// header of the ordinary invoice.
fn check_guidance_header(header: &FatturaElettronicaHeaderTypeContent, v: &mut Vec<Violation>) {
    let cedente = &header.cedente_prestatore.content;
    let cessionario = &header.cessionario_committente.content;
    let seats = [
        ("CedentePrestatore/Sede", Some(&cedente.sede)),
        (
            "CedentePrestatore/StabileOrganizzazione",
            cedente.stabile_organizzazione.as_ref(),
        ),
        ("CessionarioCommittente/Sede", Some(&cessionario.sede)),
        (
            "CessionarioCommittente/StabileOrganizzazione",
            cessionario.stabile_organizzazione.as_ref(),
        ),
    ];
    for (path, seat) in seats {
        if let Some(seat) = seat {
            let seat = &seat.content;
            check_foreign_seat(
                &format!("FatturaElettronicaHeader/{path}"),
                &seat.nazione,
                &seat.cap,
                seat.provincia.as_deref(),
                v,
            );
        }
    }
    let trasmittente = &header.dati_trasmissione.content.id_trasmittente.content;
    let seller = &cedente.dati_anagrafici.content;
    let buyer = &cessionario.dati_anagrafici.content;
    check_trasmittente(
        (&trasmittente.id_paese, &trasmittente.id_codice),
        &[
            FiscalIds {
                partita_iva: Some((
                    &seller.id_fiscale_iva.content.id_paese,
                    &seller.id_fiscale_iva.content.id_codice,
                )),
                codice_fiscale: seller.codice_fiscale.as_deref(),
            },
            FiscalIds {
                partita_iva: buyer.id_fiscale_iva.as_ref().map(|iva| {
                    (
                        iva.content.id_paese.as_str(),
                        iva.content.id_codice.as_str(),
                    )
                }),
                codice_fiscale: buyer.codice_fiscale.as_deref(),
            },
        ],
        v,
    );
}

/// The `RIFERIMENTO-NORMATIVO` and `BOLLO` rules on one body of the
/// ordinary invoice.
fn check_guidance_body(
    body: &FatturaElettronicaBodyType,
    header: &FatturaElettronicaHeaderTypeContent,
    bpath: &str,
    v: &mut Vec<Violation>,
) {
    let beni = &body.content.dati_beni_servizi.content;
    for (ri, riep) in beni.dati_riepilogo.iter().enumerate() {
        if riep.content.natura.is_some() && riep.content.riferimento_normativo.is_none() {
            v.push(Violation::new(
                "RIFERIMENTO-NORMATIVO",
                format!(
                    "{bpath}/DatiBeniServizi/DatiRiepilogo[{}]/RiferimentoNormativo",
                    ri + 1
                ),
                "RiferimentoNormativo is missing in a DatiRiepilogo with a Natura",
            ));
        }
    }
    let doc_data = &body
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content;
    if doc_data.dati_bollo.is_none()
        && bollo_due(
            &doc_data.tipo_documento,
            &header
                .cedente_prestatore
                .content
                .dati_anagrafici
                .content
                .regime_fiscale,
            beni.dettaglio_linee.iter().map(|line| &line.content),
        )
    {
        v.push(Violation::new(
            "BOLLO",
            format!("{bpath}/DatiGenerali/DatiGeneraliDocumento/DatiBollo"),
            "DatiBollo is missing while the operations subject to the stamp duty exceed 77.47 euros",
        ));
    }
}

/// The `CAP-ESTERO`, `PROVINCIA-ESTERA` and `TRASMITTENTE-CF` rules on the
/// header of the simplified invoice.
fn check_simplified_guidance_header(header: &HeaderSemplificato, v: &mut Vec<Violation>) {
    let cedente = &header.cedente_prestatore.content;
    let altri = header
        .cessionario_committente
        .content
        .altri_dati_identificativi
        .as_ref()
        .map(|altri| &altri.content);
    let seats = [
        ("CedentePrestatore/Sede", Some(&cedente.sede)),
        (
            "CedentePrestatore/StabileOrganizzazione",
            cedente.stabile_organizzazione.as_ref(),
        ),
        (
            "CessionarioCommittente/AltriDatiIdentificativi/Sede",
            altri.map(|altri| &altri.sede),
        ),
        (
            "CessionarioCommittente/AltriDatiIdentificativi/StabileOrganizzazione",
            altri.and_then(|altri| altri.stabile_organizzazione.as_ref()),
        ),
    ];
    for (path, seat) in seats {
        if let Some(seat) = seat {
            let seat = &seat.content;
            check_foreign_seat(
                &format!("FatturaElettronicaHeader/{path}"),
                &seat.nazione,
                &seat.cap,
                seat.provincia.as_deref(),
                v,
            );
        }
    }
    let trasmittente = &header.dati_trasmissione.content.id_trasmittente.content;
    let buyer = &header
        .cessionario_committente
        .content
        .identificativi_fiscali
        .content;
    check_trasmittente(
        (&trasmittente.id_paese, &trasmittente.id_codice),
        &[
            FiscalIds {
                partita_iva: Some((
                    &cedente.id_fiscale_iva.content.id_paese,
                    &cedente.id_fiscale_iva.content.id_codice,
                )),
                codice_fiscale: cedente.codice_fiscale.as_deref(),
            },
            FiscalIds {
                partita_iva: buyer.id_fiscale_iva.as_ref().map(|iva| {
                    (
                        iva.content.id_paese.as_str(),
                        iva.content.id_codice.as_str(),
                    )
                }),
                codice_fiscale: buyer.codice_fiscale.as_deref(),
            },
        ],
        v,
    );
}

/// The `RIFERIMENTO-NORMATIVO` and `BOLLO` rules on one body of the
/// simplified invoice.
fn check_simplified_guidance_body(
    bi: usize,
    body: &BodySemplificato,
    header: &HeaderSemplificato,
    v: &mut Vec<Violation>,
) {
    let bpath = format!("FatturaElettronicaBody[{}]", bi + 1);
    let beni = &body.content.dati_beni_servizi;
    for (si, item) in beni.iter().enumerate() {
        if item.content.natura.is_some() && item.content.riferimento_normativo.is_none() {
            v.push(Violation::new(
                "RIFERIMENTO-NORMATIVO",
                format!("{bpath}/DatiBeniServizi[{}]/RiferimentoNormativo", si + 1),
                "RiferimentoNormativo is missing in a DatiBeniServizi with a Natura",
            ));
        }
    }
    let doc_data = &body
        .content
        .dati_generali
        .content
        .dati_generali_documento
        .content;
    if doc_data.bollo_virtuale.is_none()
        && simplified_bollo_due(&header.cedente_prestatore.content.regime_fiscale, beni)
    {
        v.push(Violation::new(
            "BOLLO",
            format!("{bpath}/DatiGenerali/DatiGeneraliDocumento/BolloVirtuale"),
            "BolloVirtuale is missing while the operations subject to the stamp duty exceed 77.47 euros",
        ));
    }
}
