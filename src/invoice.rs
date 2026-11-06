//! The typed construction of the `FatturaElettronica` document, on top of
//! the generated types.
//!
//! The scenarios that the specification dictates are named constructors,
//! `Invoice::domestic`, `Invoice::cross_border_services` and
//! `SelfInvoice::td17` and `td19`, each with a `bon` builder. The scenario
//! parties and the first line are parameters of the constructor, the
//! `ProgressivoInvio`, the `Data` and the `Numero` are required setters,
//! so a document missing a required field does not compile. Every
//! `build` checks the scenario rules, computes the line totals, the
//! riepilogo and the total amount per the rounding rules of the
//! specification, and runs `validate::validate` before returning the
//! document.
//!
//! A document in a foreign currency keeps the `Divisa` `EUR` and every
//! amount the `SDI` reads in euros, the art. 13 convention of the Agenzia
//! FAQ. The original amount, the currency and the rate appear only as
//! management data, in the `AltriDatiGestionali` of each line.

use crate::amount::Amount;
use crate::date::Date;
use crate::generated::fpr12::{
    AltriDatiGestionaliType, AltriDatiGestionaliTypeContent, BolloVirtualeType,
    CedentePrestatoreType, CondizioniPagamentoType, DatiBeniServiziType,
    DatiBeniServiziTypeContent, DatiBolloType, DatiBolloTypeContent, DatiDocumentiCorrelatiType,
    DatiDocumentiCorrelatiTypeContent, DatiGeneraliDocumentoType, DatiGeneraliDocumentoTypeContent,
    DatiGeneraliType, DatiGeneraliTypeContent, DatiPagamentoType, DatiPagamentoTypeContent,
    DatiRiepilogoType, DatiRiepilogoTypeContent, DatiTrasmissioneType, DatiTrasmissioneTypeContent,
    DettaglioLineeType, DettaglioLineeTypeContent, DettaglioPagamentoType,
    DettaglioPagamentoTypeContent, FatturaElettronica, FatturaElettronicaBodyType,
    FatturaElettronicaBodyTypeContent, FatturaElettronicaHeaderType,
    FatturaElettronicaHeaderTypeContent, FatturaElettronicaTypeContent, FormatoTrasmissioneType,
    IdFiscaleType, IdFiscaleTypeContent, ModalitaPagamentoType, ScontoMaggiorazioneType,
    ScontoMaggiorazioneTypeContent, TipoDocumentoType, TipoScontoMaggiorazioneType,
};
use crate::ids::{
    Aliquota, Causale, CodiceDestinatario, Description, ExchangeRate, IdDocumento, Natura, Numero,
    Pec, ProgressivoInvio, RegimeFiscale, RiferimentoNormativo, UnitaMisura,
};
use crate::party::{Business, Buyer, Company, Party, Seller};
use crate::validate::{self, Violation};
use bon::bon;
use iso_currency::Currency;
use rust_decimal::Decimal;

/// The `CodiceDestinatario` the `SDI` assigns to a non-resident buyer.
const FOREIGN_CODICE_DESTINATARIO: &str = "XXXXXXX";

/// The `CodiceDestinatario` written with a `PECDestinatario` or an unknown
/// channel.
const PEC_CODICE_DESTINATARIO: &str = "0000000";

/// The 77.47 EUR stamp duty threshold of the DPR 642/72.
const BOLLO_THRESHOLD: Decimal = Decimal::from_parts(7747, 0, 0, false, 2);

/// The 2.00 EUR amount of the virtual stamp duty.
const BOLLO_IMPORT: Decimal = Decimal::from_parts(200, 0, 0, false, 2);

/// The thirty days from the document date of the self-invoice payment.
const PAYMENT_DAYS: i32 = 30;

/// The exclusive bound of the `PrezzoUnitario`, the `Amount8DecimalType`
/// integer part of at most nine digits.
fn unit_price_bound() -> Decimal {
    Decimal::new(1_000_000_000, 0)
}

/// The exclusive bound of the `Quantita`, the `QuantitaType` integer part
/// of at most twelve digits.
fn quantity_bound() -> Decimal {
    Decimal::new(1_000_000_000_000, 0)
}

/// What a `build` refuses when the document cannot be assembled.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BuildError {
    /// The amount of a line does not fit the XSD amount types of the
    /// document.
    ///
    /// `line` is the one-based line number, `field` the amount field and
    /// `value` the offending value.
    #[error("line {line} {field} is not a valid amount: {value}")]
    Amount {
        /// The one-based line number.
        line: i32,
        /// The amount field of the line.
        field: AmountField,
        /// The offending value.
        value: Decimal,
    },
    /// The assembled document fails one or more `SDI` checks.
    #[error("the document fails {} SDI check(s)", .0.len())]
    Violations(Vec<Violation>),
}

/// The amount field of a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmountField {
    /// The `PrezzoUnitario`.
    UnitPrice,
    /// The `Quantita`.
    Quantity,
}

impl std::fmt::Display for AmountField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnitPrice => f.write_str("the unit price"),
            Self::Quantity => f.write_str("the quantity"),
        }
    }
}

/// The channel the `SDI` uses to reach the buyer of a domestic invoice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recipient {
    /// The `CodiceDestinatario` of the buyer, seven characters for the
    /// `FPR12` format and six characters for the `FPA12` format.
    Codice(CodiceDestinatario),
    /// The `PECDestinatario` of the buyer, which writes the
    /// `CodiceDestinatario` `0000000`.
    Pec(Pec),
    /// No known channel, which writes the `CodiceDestinatario` `0000000`.
    Unknown,
}

/// The `AliquotaIVA` and the `Natura` of a line, one of the two, so a
/// taxed nature or a non-taxed rate with a percentage cannot be written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VatTreatment {
    /// A taxable line, with the `AliquotaIVA` the rate.
    Taxed(Aliquota),
    /// A non-taxable line, with the `AliquotaIVA` `0.00`, the `Natura` the
    /// nature and the `RiferimentoNormativo` of the riepilogo.
    NotTaxed {
        /// The `Natura` of the operation.
        natura: Natura,
        /// The `RiferimentoNormativo` the riepilogo carries with the
        /// nature.
        riferimento_normativo: RiferimentoNormativo,
    },
}

impl VatTreatment {
    /// The `AliquotaIVA` the line and the riepilogo carry.
    pub(crate) fn rate(&self) -> Decimal {
        match self {
            Self::Taxed(aliquota) => aliquota.as_decimal(),
            Self::NotTaxed { .. } => Decimal::new(0, 2),
        }
    }

    /// The `Natura` of the non-taxable line.
    pub(crate) const fn natura(&self) -> Option<&Natura> {
        match self {
            Self::Taxed(..) => None,
            Self::NotTaxed { natura, .. } => Some(natura),
        }
    }

    /// The `RiferimentoNormativo` of the non-taxable line.
    pub(crate) const fn riferimento_normativo(&self) -> Option<&RiferimentoNormativo> {
        match self {
            Self::Taxed(..) => None,
            Self::NotTaxed {
                riferimento_normativo,
                ..
            } => Some(riferimento_normativo),
        }
    }
}

/// The currency of the document.
///
/// The `Eur` form writes the `Divisa` `EUR` and the amounts as given. The
/// `Foreign` form is a document priced in a foreign currency, whose line
/// prices the caller gives in `currency`. The `SDI` reads every amount in
/// euros, the art. 13 convention of the Agenzia FAQ, so `build` converts
/// the prices with the rate, writes the `Divisa` `EUR`, and records the
/// original amount, the currency and the rate in the
/// `AltriDatiGestionali` of each line. The `rate` is the units of
/// `currency` per euro, the European Central Bank convention of art. 13.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InvoiceCurrency {
    /// The `Divisa` `EUR`, the amounts as given.
    #[default]
    Eur,
    /// A document priced in a foreign currency.
    Foreign {
        /// The currency of the original prices.
        currency: Currency,
        /// The units of `currency` per euro, of the `rate_date`.
        rate: ExchangeRate,
        /// The date of the rate.
        rate_date: Date,
    },
}

impl InvoiceCurrency {
    /// The euro value of a foreign amount, the art. 13 conversion rounded
    /// to the cent.
    pub(crate) fn eur_value(&self, value: Decimal) -> Decimal {
        match self {
            Self::Eur => value,
            Self::Foreign { rate, .. } => validate::round_half_2(value / rate.as_decimal()),
        }
    }
}

/// A `ScontoMaggiorazione` block, either a percentage or an amount, of the
/// discount or the markup kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sconto {
    /// Whether the block is the `SC` discount or the `MG` markup.
    discount: bool,
    /// Whether the value is the `Percentuale` or the `Importo`.
    percent: bool,
    /// The value, at most eight decimals.
    value: Decimal,
}

impl Sconto {
    /// A discount of `value` percent, `SC` and `Percentuale`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the value is not a positive percentage of at
    /// most 100 with at most two decimals.
    pub fn discount_percent(value: impl Into<Decimal>) -> Result<Self, crate::ids::IdError> {
        Self::new(true, true, value)
    }

    /// A markup of `value` percent, `MG` and `Percentuale`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the value is not a positive percentage of at
    /// most 100 with at most two decimals.
    pub fn markup_percent(value: impl Into<Decimal>) -> Result<Self, crate::ids::IdError> {
        Self::new(false, true, value)
    }

    /// A discount of `value` euros, `SC` and `Importo`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the value is not a positive amount of the
    /// `Amount8DecimalType` bounds.
    pub fn discount_amount(value: impl Into<Decimal>) -> Result<Self, crate::ids::IdError> {
        Self::new(true, false, value)
    }

    /// A markup of `value` euros, `MG` and `Importo`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the value is not a positive amount of the
    /// `Amount8DecimalType` bounds.
    pub fn markup_amount(value: impl Into<Decimal>) -> Result<Self, crate::ids::IdError> {
        Self::new(false, false, value)
    }

    /// Validates the value and builds the block. The `Percentuale` fits
    /// the `RateType` of at most two decimals, the `Importo` the
    /// `Amount8DecimalType` of at most eight decimals and nine integer
    /// digits.
    fn new(
        discount: bool,
        percent: bool,
        value: impl Into<Decimal>,
    ) -> Result<Self, crate::ids::IdError> {
        let original = value.into();
        let mut value = original;
        let max_scale = if percent { 2 } else { 8 };
        value.rescale(max_scale);
        if value != original
            || value <= Decimal::ZERO
            || (percent && value > Decimal::ONE_HUNDRED)
            || (!percent && value >= unit_price_bound())
        {
            return Err(crate::ids::IdError::Sconto(original.to_string()));
        }
        Ok(Self {
            discount,
            percent,
            value,
        })
    }

    /// The generated `ScontoMaggiorazione` element, the `Importo`
    /// converted to euros when the document reads a foreign currency. The
    /// `Percentuale` is currency-neutral and stays as is.
    pub(crate) fn sconto_maggiorazione(
        &self,
        currency: &InvoiceCurrency,
    ) -> ScontoMaggiorazioneType {
        let (percentuale, importo) = if self.percent {
            (Some(Amount(norm2(self.value))), None)
        } else {
            (None, Some(Amount(currency.eur_value(norm2(self.value)))))
        };
        ScontoMaggiorazioneType {
            content: ScontoMaggiorazioneTypeContent {
                tipo: self.tipo(),
                percentuale,
                importo,
            },
        }
    }

    /// The `Tipo` of the block.
    const fn tipo(&self) -> TipoScontoMaggiorazioneType {
        if self.discount {
            TipoScontoMaggiorazioneType::Sc
        } else {
            TipoScontoMaggiorazioneType::Mg
        }
    }
}

/// One `DettaglioLinee` of a typed invoice.
///
/// The `NumeroLinea` is assigned by the invoice at `build`, in document
/// order. The `PrezzoUnitario` is in the currency the invoice carries, and
/// the `PrezzoTotale` is computed by the invoice from the unit price, the
/// quantity and the `ScontoMaggiorazione` blocks, per the 00423 rule.
#[derive(bon::Builder, Clone, Debug)]
pub struct Line {
    /// The `Descrizione`.
    description: Description,
    /// The `Quantita`, absent when the line is not quantity-priced and the
    /// total reads the unit price.
    quantity: Option<Amount>,
    /// The `UnitaMisura`.
    unit_of_measure: Option<UnitaMisura>,
    /// The `PrezzoUnitario`, in the currency the invoice carries.
    unit_price: Amount,
    /// The `ScontoMaggiorazione` blocks, in document order.
    #[builder(default)]
    sconti: Vec<Sconto>,
    /// The `AliquotaIVA` or the `Natura` of the line.
    vat: VatTreatment,
}

impl Line {
    /// The VAT treatment of the line.
    #[must_use]
    pub const fn vat(&self) -> &VatTreatment {
        &self.vat
    }

    /// The `Natura` of the non-taxable line.
    #[must_use]
    pub const fn natura(&self) -> Option<&Natura> {
        self.vat.natura()
    }

    /// The line in the generated types, with its `PrezzoTotale` computed
    /// and its amounts in the currency the document reads.
    pub(crate) fn into_content(
        self,
        numero_linea: i32,
        currency: &InvoiceCurrency,
    ) -> Result<DettaglioLineeTypeContent, BuildError> {
        let unit_price = currency.eur_value(amount_in_range(
            self.unit_price.0,
            AmountField::UnitPrice,
            numero_linea,
        )?);
        let sconti = self
            .sconti
            .iter()
            .map(|sconto| sconto.sconto_maggiorazione(currency))
            .collect();
        let quantita = match self.quantity {
            Some(quantity) => Some(Amount(amount_in_range(
                quantity.0,
                AmountField::Quantity,
                numero_linea,
            )?)),
            None => None,
        };
        let mut content = DettaglioLineeTypeContent {
            numero_linea,
            tipo_cessione_prestazione: None,
            codice_articolo: Vec::new(),
            descrizione: self.description.as_str().to_string(),
            quantita,
            unita_misura: self.unit_of_measure.map(|unit| unit.as_str().to_string()),
            data_inizio_periodo: None,
            data_fine_periodo: None,
            prezzo_unitario: Amount(unit_price),
            sconto_maggiorazione: sconti,
            prezzo_totale: Amount(Decimal::ZERO),
            aliquota_iva: Amount(self.vat.rate()),
            ritenuta: None,
            natura: self.vat.natura().map(|natura| natura.generated()),
            riferimento_amministrazione: None,
            altri_dati_gestionali: currency.altri_dati_gestionali(&self.unit_price),
        };
        content.prezzo_totale = validate::line_total(&content);
        Ok(content)
    }
}

impl InvoiceCurrency {
    /// The `AltriDatiGestionali` of a foreign-currency line, the original
    /// amount and the rate.
    fn altri_dati_gestionali(&self, original: &Amount) -> Vec<AltriDatiGestionaliType> {
        match self {
            Self::Eur => Vec::new(),
            Self::Foreign {
                currency,
                rate,
                rate_date,
            } => {
                let code = currency.code();
                vec![
                    AltriDatiGestionaliType {
                        content: AltriDatiGestionaliTypeContent {
                            tipo_dato: format!("{code} AMT"),
                            riferimento_testo: Some(format!("Original amount in {code}")),
                            riferimento_numero: Some(*original),
                            riferimento_data: None,
                        },
                    },
                    AltriDatiGestionaliType {
                        content: AltriDatiGestionaliTypeContent {
                            tipo_dato: format!("{code} RATE"),
                            riferimento_testo: Some(format!("{code} units per euro")),
                            riferimento_numero: Some(Amount(rate.as_decimal())),
                            riferimento_data: Some(*rate_date),
                        },
                    },
                ]
            }
        }
    }
}

/// The `RiferimentoNormativo` the art. 7-ter scenario writes on every line
/// and riepilogo.
const SEVEN_TER_REFERENCE: &str = "ART. 7-TER DPR 633/72";

/// The `VatTreatment` of the art. 7-ter line, the `N2.1` nature and the
/// normative reference the specification dictates for the scenario.
#[expect(
    clippy::expect_used,
    reason = "the reference is a constant checked in against its rule"
)]
fn seven_ter_treatment() -> VatTreatment {
    VatTreatment::NotTaxed {
        natura: Natura::N2_1,
        riferimento_normativo: RiferimentoNormativo::try_new(SEVEN_TER_REFERENCE)
            .expect("the constant reference is valid"),
    }
}

/// One `DettaglioLinee` of the art. 7-ter invoice.
///
/// The scenario writes the `N2.1` nature and the `ART. 7-TER DPR 633/72`
/// reference on every line, so the line carries no VAT treatment of its
/// own. The `NumeroLinea` is assigned by the invoice at `build`, in
/// document order. The `PrezzoUnitario` is in the currency the invoice
/// carries, and the `PrezzoTotale` is computed by the invoice from the
/// unit price, the quantity and the `ScontoMaggiorazione` blocks, per the
/// 00423 rule.
#[derive(bon::Builder, Clone, Debug)]
pub struct SevenTerLine {
    /// The `Descrizione`.
    description: Description,
    /// The `Quantita`, absent when the line is not quantity-priced and the
    /// total reads the unit price.
    quantity: Option<Amount>,
    /// The `UnitaMisura`.
    unit_of_measure: Option<UnitaMisura>,
    /// The `PrezzoUnitario`, in the currency the invoice carries.
    unit_price: Amount,
    /// The `ScontoMaggiorazione` blocks, in document order.
    #[builder(default)]
    sconti: Vec<Sconto>,
}

impl SevenTerLine {
    /// The line in the `Line` form the assembly reads, with the `N2.1`
    /// nature and the `ART. 7-TER DPR 633/72` reference of the scenario.
    fn into_line(self) -> Line {
        Line {
            description: self.description,
            quantity: self.quantity,
            unit_of_measure: self.unit_of_measure,
            unit_price: self.unit_price,
            sconti: self.sconti,
            vat: seven_ter_treatment(),
        }
    }
}

/// The `Line` form of the art. 7-ter lines, in document order.
fn seven_ter_lines(lines: Vec<SevenTerLine>) -> Vec<Line> {
    lines.into_iter().map(SevenTerLine::into_line).collect()
}

/// The first line and the remaining lines of the art. 7-ter scenario.
fn first_seven_ter_lines(first: SevenTerLine, rest: Vec<SevenTerLine>) -> Vec<SevenTerLine> {
    let mut lines = vec![first];
    lines.extend(rest);
    lines
}

/// The scenario the `Invoice` constructor fills.
#[derive(Clone, Debug)]
enum Scenario {
    /// The domestic transmission, with its recipient channel.
    Domestic {
        /// The `CodiceDestinatario` or `PECDestinatario` channel of the
        /// buyer.
        recipient: Recipient,
    },
    /// The art. 7-ter transmission to a foreign client.
    CrossBorder,
}

/// A `FatturaElettronica` of one seller and one buyer, in the scenario a
/// named constructor fills.
pub struct Invoice;

#[bon]
impl Invoice {
    /// The domestic `TD01` invoice of a resident seller. The `FPR12`
    /// format carries the seven-character `Recipient::Codice`, the
    /// `FPA12` the six-character code of a public administration.
    ///
    /// # Errors
    ///
    /// Returns `Err` when a line amount does not fit the XSD amount
    /// types, or with every `SDI` violation when the assembled document
    /// fails a check.
    #[builder(finish_fn = build)]
    pub fn domestic(
        /// The seller of the invoice.
        #[builder(start_fn)]
        seller: &Seller,
        /// The buyer of the invoice.
        #[builder(start_fn)]
        buyer: &Buyer,
        /// The recipient channel of the buyer.
        #[builder(start_fn)]
        recipient: Recipient,
        /// The first line of the invoice.
        #[builder(start_fn)]
        line: Line,
        /// The `ProgressivoInvio` of the transmission.
        progressivo: ProgressivoInvio,
        /// The `Data` of the document.
        date: Date,
        /// The `Numero` of the document.
        numero: Numero,
        /// The remaining lines, in document order.
        #[builder(default)]
        lines: Vec<Line>,
        /// The `Causale` texts of the document.
        #[builder(default)]
        causali: Vec<Causale>,
        /// The currency of the document.
        #[builder(default)]
        currency: InvoiceCurrency,
        /// Enables the virtual stamp duty, the 2.00 euros amount when the
        /// computed total exceeds 77.47 euros.
        #[builder(default, with = || true)]
        bollo: bool,
    ) -> Result<FatturaElettronica, BuildError> {
        build_invoice(
            seller.cedente(),
            seller.id_trasmittente(),
            buyer.party(),
            &Scenario::Domestic { recipient },
            TipoDocumentoType::Td01,
            progressivo,
            date,
            numero,
            causali,
            bollo,
            first_lines(line, lines),
            currency,
            Vec::new(),
            Vec::new(),
        )
    }

    /// The art. 7-ter services invoice to an EU or non-EU business
    /// client. The `FPR12` format and the `XXXXXXX` recipient code are
    /// dictated by the scenario, every line carries the `N2.1` nature and
    /// the `ART. 7-TER DPR 633/72` reference the scenario writes, and the
    /// virtual stamp duty is written when the total exceeds 77.47 euros.
    ///
    /// # Errors
    ///
    /// Returns `Err` when a line amount does not fit the XSD amount
    /// types, or with every `SDI` violation when the assembled document
    /// fails a check.
    #[builder(finish_fn = build)]
    pub fn cross_border_services(
        /// The seller of the invoice.
        #[builder(start_fn)]
        seller: &Seller,
        /// The EU or non-EU business client of the invoice.
        #[builder(start_fn)]
        buyer: &Business,
        /// The first line of the invoice.
        #[builder(start_fn)]
        line: SevenTerLine,
        /// The `ProgressivoInvio` of the transmission.
        progressivo: ProgressivoInvio,
        /// The `Data` of the document.
        date: Date,
        /// The `Numero` of the document.
        numero: Numero,
        /// The remaining lines, in document order.
        #[builder(default)]
        lines: Vec<SevenTerLine>,
        /// The `Causale` texts of the document.
        #[builder(default)]
        causali: Vec<Causale>,
        /// The currency of the document.
        #[builder(default)]
        currency: InvoiceCurrency,
    ) -> Result<FatturaElettronica, BuildError> {
        build_invoice(
            seller.cedente(),
            seller.id_trasmittente(),
            buyer.party(),
            &Scenario::CrossBorder,
            TipoDocumentoType::Td01,
            progressivo,
            date,
            numero,
            causali,
            true,
            seven_ter_lines(first_seven_ter_lines(line, lines)),
            currency,
            Vec::new(),
            Vec::new(),
        )
    }
}

/// A self-invoice of a buyer integrating the reverse charge of a foreign
/// supplier, the `TD17` of services and the `TD19` of goods.
pub struct SelfInvoice;

#[bon]
impl SelfInvoice {
    /// The `TD17` self-invoice of a service bought from a non-Italian
    /// supplier, the reverse charge of the art. 7-ter. The `FPR12` or
    /// `FPA12` format, the `IdTrasmittente` of the Italian buyer, the
    /// `RF01` regime the XSD demands of the foreign supplier, and the
    /// `DatiPagamento` at the buyer's terms are dictated by the scenario.
    ///
    /// # Errors
    ///
    /// Returns `Err` when a line amount does not fit the XSD amount
    /// types, or with every `SDI` violation when the assembled document
    /// fails a check.
    #[builder(finish_fn = build)]
    pub fn td17(
        /// The non-Italian supplier of the service.
        #[builder(start_fn)]
        supplier: &Business,
        /// The Italian buyer with its `partita IVA`.
        #[builder(start_fn)]
        buyer: &Company,
        /// The recipient channel of the buyer.
        #[builder(start_fn)]
        recipient: Recipient,
        /// The first line of the self-invoice, at the buyer's rate.
        #[builder(start_fn)]
        line: Line,
        /// The `ProgressivoInvio` of the transmission.
        progressivo: ProgressivoInvio,
        /// The `Data` of the document.
        date: Date,
        /// The `Numero` of the document.
        numero: Numero,
        /// The remaining lines, in document order.
        #[builder(default)]
        lines: Vec<Line>,
        /// The `Causale` texts of the document.
        #[builder(default)]
        causali: Vec<Causale>,
        /// The currency of the document.
        #[builder(default)]
        currency: InvoiceCurrency,
        /// The `DatiFattureCollegate` to the original document, its
        /// `IdDocumento` and `Data`.
        #[builder(with = |numero: IdDocumento, data: Date| (numero, data), name = original_invoice)]
        original: Option<(IdDocumento, Date)>,
    ) -> Result<FatturaElettronica, BuildError> {
        build_invoice(
            supplier.cedente(RegimeFiscale::Rf01),
            buyer.id_trasmittente(),
            buyer.party(),
            &Scenario::Domestic { recipient },
            TipoDocumentoType::Td17,
            progressivo,
            date,
            numero,
            causali,
            false,
            first_lines(line, lines),
            currency,
            vec![pagamento(date)],
            original
                .map(|(numero, data)| collegate(&numero, data))
                .into_iter()
                .collect(),
        )
    }

    /// The `TD19` self-invoice of goods bought from a non-Italian
    /// supplier, the reverse charge of the art. 7-ter.
    ///
    /// # Errors
    ///
    /// Returns `Err` when a line amount does not fit the XSD amount
    /// types, or with every `SDI` violation when the assembled document
    /// fails a check.
    #[builder(finish_fn = build)]
    pub fn td19(
        /// The non-Italian supplier of the goods.
        #[builder(start_fn)]
        supplier: &Business,
        /// The Italian buyer with its `partita IVA`.
        #[builder(start_fn)]
        buyer: &Company,
        /// The recipient channel of the buyer.
        #[builder(start_fn)]
        recipient: Recipient,
        /// The first line of the self-invoice, at the buyer's rate.
        #[builder(start_fn)]
        line: Line,
        /// The `ProgressivoInvio` of the transmission.
        progressivo: ProgressivoInvio,
        /// The `Data` of the document.
        date: Date,
        /// The `Numero` of the document.
        numero: Numero,
        /// The remaining lines, in document order.
        #[builder(default)]
        lines: Vec<Line>,
        /// The `Causale` texts of the document.
        #[builder(default)]
        causali: Vec<Causale>,
        /// The currency of the document.
        #[builder(default)]
        currency: InvoiceCurrency,
        /// The `DatiFattureCollegate` to the original document, its
        /// `IdDocumento` and `Data`.
        #[builder(with = |numero: IdDocumento, data: Date| (numero, data), name = original_invoice)]
        original: Option<(IdDocumento, Date)>,
    ) -> Result<FatturaElettronica, BuildError> {
        build_invoice(
            supplier.cedente(RegimeFiscale::Rf01),
            buyer.id_trasmittente(),
            buyer.party(),
            &Scenario::Domestic { recipient },
            TipoDocumentoType::Td19,
            progressivo,
            date,
            numero,
            causali,
            false,
            first_lines(line, lines),
            currency,
            vec![pagamento(date)],
            original
                .map(|(numero, data)| collegate(&numero, data))
                .into_iter()
                .collect(),
        )
    }
}

/// The checks and the assembly shared by the invoice and the self-invoice.
#[expect(
    clippy::too_many_arguments,
    reason = "the shared assembly takes every field of the four scenario builders"
)]
fn build_invoice(
    cedente: CedentePrestatoreType,
    id_trasmittente: (String, String),
    buyer: &Party,
    scenario: &Scenario,
    tipo: TipoDocumentoType,
    progressivo: ProgressivoInvio,
    date: Date,
    numero: Numero,
    causali: Vec<Causale>,
    bollo: bool,
    lines: Vec<Line>,
    currency: InvoiceCurrency,
    dati_pagamento: Vec<DatiPagamentoType>,
    collegate: Vec<DatiDocumentiCorrelatiType>,
) -> Result<FatturaElettronica, BuildError> {
    let (formato, codice_destinatario, pec_destinatario) = recipient_of(scenario);
    let header = header(
        formato.clone(),
        progressivo,
        codice_destinatario,
        pec_destinatario,
        id_trasmittente,
        cedente,
        buyer,
    );
    let body = assemble_body(BodyInputs {
        tipo,
        data: date,
        numero,
        causale: causali,
        bollo,
        lines,
        currency,
        dati_pagamento,
        collegate,
    })?;
    let doc = FatturaElettronica {
        versione: formato,
        sistema_emittente: None,
        content: FatturaElettronicaTypeContent {
            fattura_elettronica_header: header,
            fattura_elettronica_body: vec![body],
            signature: None,
        },
    };
    let violations = validate::validate(&doc);
    if violations.is_empty() {
        Ok(doc)
    } else {
        Err(BuildError::Violations(violations))
    }
}

/// The format, the `CodiceDestinatario` and the optional
/// `PECDestinatario` of the scenario.
fn recipient_of(scenario: &Scenario) -> (FormatoTrasmissioneType, String, Option<String>) {
    match scenario {
        Scenario::Domestic { recipient } => match recipient {
            Recipient::Codice(codice) => (formato_of(codice), codice.as_str().to_string(), None),
            Recipient::Pec(pec) => (
                FormatoTrasmissioneType::Fpr12,
                PEC_CODICE_DESTINATARIO.to_string(),
                Some(pec.as_str().to_string()),
            ),
            Recipient::Unknown => (
                FormatoTrasmissioneType::Fpr12,
                PEC_CODICE_DESTINATARIO.to_string(),
                None,
            ),
        },
        Scenario::CrossBorder => (
            FormatoTrasmissioneType::Fpr12,
            FOREIGN_CODICE_DESTINATARIO.to_string(),
            None,
        ),
    }
}

/// The transmission format of a recipient code, `FPA12` for the six
/// characters of a public administration, `FPR12` for the seven.
const fn formato_of(codice: &CodiceDestinatario) -> FormatoTrasmissioneType {
    if codice.is_public_administration() {
        FormatoTrasmissioneType::Fpa12
    } else {
        FormatoTrasmissioneType::Fpr12
    }
}

/// The `DatiPagamento` of the self-invoice, at the buyer's terms.
fn pagamento(data: Date) -> DatiPagamentoType {
    let scadenza = Date(data.0 + jiff::Span::new().days(PAYMENT_DAYS));
    DatiPagamentoType {
        content: DatiPagamentoTypeContent {
            condizioni_pagamento: CondizioniPagamentoType::Tp01,
            dettaglio_pagamento: vec![DettaglioPagamentoType {
                content: DettaglioPagamentoTypeContent {
                    beneficiario: None,
                    modalita_pagamento: ModalitaPagamentoType::Mp01,
                    data_riferimento_termini_pagamento: None,
                    giorni_termini_pagamento: None,
                    data_scadenza_pagamento: Some(scadenza),
                    // The total is filled by the caller after the riepilogo.
                    importo_pagamento: Amount(Decimal::ZERO),
                    cod_ufficio_postale: None,
                    cognome_quietanzante: None,
                    nome_quietanzante: None,
                    cf_quietanzante: None,
                    titolo_quietanzante: None,
                    istituto_finanziario: None,
                    iban: None,
                    abi: None,
                    cab: None,
                    bic: None,
                    sconto_pagamento_anticipato: None,
                    data_limite_pagamento_anticipato: None,
                    penalita_pagamenti_ritardati: None,
                    data_decorrenza_penale: None,
                    codice_pagamento: None,
                },
            }],
        },
    }
}

/// The `DatiFattureCollegate` to the original document.
fn collegate(numero: &IdDocumento, data: Date) -> DatiDocumentiCorrelatiType {
    DatiDocumentiCorrelatiType {
        content: DatiDocumentiCorrelatiTypeContent {
            riferimento_numero_linea: Vec::new(),
            id_documento: numero.as_str().to_string(),
            data: Some(data),
            num_item: None,
            codice_commessa_convenzione: None,
            codice_cup: None,
            codice_cig: None,
        },
    }
}

/// The header of the assembled invoice.
fn header(
    formato: FormatoTrasmissioneType,
    progressivo: ProgressivoInvio,
    codice_destinatario: String,
    pec_destinatario: Option<String>,
    id_trasmittente: (String, String),
    cedente: CedentePrestatoreType,
    buyer: &Party,
) -> FatturaElettronicaHeaderType {
    FatturaElettronicaHeaderType {
        content: FatturaElettronicaHeaderTypeContent {
            dati_trasmissione: DatiTrasmissioneType {
                content: DatiTrasmissioneTypeContent {
                    id_trasmittente: IdFiscaleType {
                        content: IdFiscaleTypeContent {
                            id_paese: id_trasmittente.0,
                            id_codice: id_trasmittente.1,
                        },
                    },
                    progressivo_invio: progressivo.into_string(),
                    formato_trasmissione: formato,
                    codice_destinatario,
                    contatti_trasmittente: None,
                    pec_destinatario,
                },
            },
            cedente_prestatore: cedente,
            rappresentante_fiscale: None,
            cessionario_committente: buyer.cessionario(),
            terzo_intermediario_o_soggetto_emittente: None,
            soggetto_emittente: None,
        },
    }
}

/// The inputs of the body assembly, shared by the scenario builders.
struct BodyInputs {
    /// The `TipoDocumento`.
    tipo: TipoDocumentoType,
    /// The `Data` of the document.
    data: Date,
    /// The `Numero` of the document.
    numero: Numero,
    /// The `Causale` texts of the document.
    causale: Vec<Causale>,
    /// Whether to write the virtual stamp duty.
    bollo: bool,
    /// The lines of the document, in document order.
    lines: Vec<Line>,
    /// The currency of the document.
    currency: InvoiceCurrency,
    /// The `DatiPagamento` of the document.
    dati_pagamento: Vec<DatiPagamentoType>,
    /// The `DatiFattureCollegate` of the document.
    collegate: Vec<DatiDocumentiCorrelatiType>,
}

/// The lines of the document and the per-rate groups of the assembly.
struct Lines {
    /// The `DettaglioLinee` content, in document order.
    contents: Vec<DettaglioLineeTypeContent>,
    /// The groups by `AliquotaIVA` and `Natura`, with their raw total.
    groups: Vec<(Decimal, Option<Natura>, Decimal)>,
    /// The `RiferimentoNormativo` of each nature.
    references: Vec<(Natura, RiferimentoNormativo)>,
}

/// Computes the `PrezzoTotale` of each line and the per-rate groups of the
/// document.
///
/// # Errors
///
/// Returns `Err` when a line amount does not fit the XSD amount types.
fn collect_lines(lines: Vec<Line>, currency: &InvoiceCurrency) -> Result<Lines, BuildError> {
    let mut contents: Vec<DettaglioLineeTypeContent> = Vec::new();
    let mut groups: Vec<(Decimal, Option<Natura>, Decimal)> = Vec::new();
    let mut references: Vec<(Natura, RiferimentoNormativo)> = Vec::new();
    for (i, line) in lines.into_iter().enumerate() {
        let natura = line.vat().natura().copied();
        let reference = line.vat().riferimento_normativo().cloned();
        let content = line.into_content(line_number(i), currency)?;
        let key = (content.aliquota_iva.0, natura);
        match groups
            .iter_mut()
            .find(|(aliquota, group_natura, _)| *aliquota == key.0 && *group_natura == key.1)
        {
            Some(group) => group.2 += content.prezzo_totale.0,
            None => groups.push((key.0, natura, content.prezzo_totale.0)),
        }
        if let (Some(natura), Some(reference)) = (natura, reference) {
            references.retain(|(n, _)| *n != natura);
            references.push((natura, reference));
        }
        contents.push(content);
    }
    Ok(Lines {
        contents,
        groups,
        references,
    })
}

/// The `DatiRiepilogo` of the per-rate groups and their total, which the
/// caller rounds to the `ImportoTotaleDocumento`.
fn riepilogo(
    groups: Vec<(Decimal, Option<Natura>, Decimal)>,
    references: &[(Natura, RiferimentoNormativo)],
) -> (Vec<DatiRiepilogoType>, Decimal) {
    let mut riep: Vec<DatiRiepilogoType> = Vec::new();
    let mut totale = Decimal::ZERO;
    for (aliquota, natura, raw) in groups {
        let imponibile = validate::round_half_2(raw);
        let arrotondamento = imponibile - raw;
        let imposta = validate::round_half_2(aliquota * imponibile / Decimal::ONE_HUNDRED);
        totale += imponibile + imposta;
        let riferimento_normativo = natura
            .as_ref()
            .and_then(|natura| references.iter().find(|(n, _)| n == natura))
            .map(|(_, text)| text.as_str().to_string());
        riep.push(DatiRiepilogoType {
            content: DatiRiepilogoTypeContent {
                aliquota_iva: Amount(aliquota),
                natura: natura.map(Natura::generated),
                spese_accessorie: None,
                arrotondamento: (arrotondamento != Decimal::ZERO).then_some(Amount(arrotondamento)),
                imponibile_importo: Amount(imponibile),
                imposta: Amount(imposta),
                esigibilita_iva: None,
                riferimento_normativo,
            },
        });
    }
    (riep, totale)
}

/// Assembles the body, computing the line totals, the riepilogo and the
/// total amount of the document.
fn assemble_body(inputs: BodyInputs) -> Result<FatturaElettronicaBodyType, BuildError> {
    let BodyInputs {
        tipo,
        data,
        numero,
        causale,
        bollo,
        lines,
        currency,
        dati_pagamento,
        collegate,
    } = inputs;
    let Lines {
        contents,
        groups,
        references,
    } = collect_lines(lines, &currency)?;
    let (riep, totale) = riepilogo(groups, &references);
    let importo_totale = validate::round_half_2(totale);
    let bollo = if bollo {
        let importo = (importo_totale > BOLLO_THRESHOLD).then_some(Amount(BOLLO_IMPORT));
        Some(DatiBolloType {
            content: DatiBolloTypeContent {
                bollo_virtuale: BolloVirtualeType::Si,
                importo_bollo: importo,
            },
        })
    } else {
        None
    };
    let dati_pagamento = fill_pagamento(dati_pagamento, importo_totale);
    Ok(FatturaElettronicaBodyType {
        content: FatturaElettronicaBodyTypeContent {
            dati_generali: DatiGeneraliType {
                content: DatiGeneraliTypeContent {
                    dati_generali_documento: DatiGeneraliDocumentoType {
                        content: DatiGeneraliDocumentoTypeContent {
                            tipo_documento: tipo,
                            divisa: "EUR".to_string(),
                            data,
                            numero: numero.as_str().to_string(),
                            dati_ritenuta: Vec::new(),
                            dati_bollo: bollo,
                            dati_cassa_previdenziale: Vec::new(),
                            sconto_maggiorazione: Vec::new(),
                            importo_totale_documento: Some(Amount(importo_totale)),
                            arrotondamento: None,
                            causale: causale
                                .into_iter()
                                .map(|causale| causale.as_str().to_string())
                                .collect(),
                            art_73: None,
                        },
                    },
                    dati_ordine_acquisto: Vec::new(),
                    dati_contratto: Vec::new(),
                    dati_convenzione: Vec::new(),
                    dati_ricezione: Vec::new(),
                    dati_fatture_collegate: collegate,
                    dati_sal: Vec::new(),
                    dati_ddt: Vec::new(),
                    dati_trasporto: None,
                    fattura_principale: None,
                },
            },
            dati_beni_servizi: DatiBeniServiziType {
                content: DatiBeniServiziTypeContent {
                    dettaglio_linee: contents
                        .into_iter()
                        .map(|content| DettaglioLineeType { content })
                        .collect(),
                    dati_riepilogo: riep,
                },
            },
            dati_veicoli: None,
            dati_pagamento,
            allegati: Vec::new(),
        },
    })
}

/// Fills the `ImportoPagamento` of the self-invoice payment with the total.
fn fill_pagamento(
    mut dati_pagamento: Vec<DatiPagamentoType>,
    totale: Decimal,
) -> Vec<DatiPagamentoType> {
    if let Some(pagamento) = dati_pagamento.first_mut()
        && let Some(dettaglio) = pagamento.content.dettaglio_pagamento.first_mut()
    {
        dettaglio.content.importo_pagamento = Amount(norm2(totale));
    }
    dati_pagamento
}

/// The first line and the remaining lines of the document.
fn first_lines(first: Line, rest: Vec<Line>) -> Vec<Line> {
    let mut lines = vec![first];
    lines.extend(rest);
    lines
}

/// The line number of the document, one-based.
fn line_number(i: usize) -> i32 {
    debug_assert!(
        i < i32::MAX as usize,
        "a document carries at most i32::MAX lines"
    );
    i32::try_from(i).unwrap_or_else(|_| panic!("a document carries at most i32::MAX lines")) + 1
}

/// Normalizes an amount the schema must read, to at least two decimals,
/// and refuses the value outside the XSD amount types of the line. The
/// line index is range-checked in `line_number`.
fn amount_in_range(value: Decimal, field: AmountField, line: i32) -> Result<Decimal, BuildError> {
    let bound = match field {
        AmountField::UnitPrice => unit_price_bound(),
        AmountField::Quantity => quantity_bound(),
    };
    let in_range = match field {
        AmountField::UnitPrice => value.abs() < bound,
        // The `QuantitaType` is non-negative.
        AmountField::Quantity => (Decimal::ZERO..bound).contains(&value),
    };
    if value.scale() > 8 || !in_range {
        Err(BuildError::Amount { line, field, value })
    } else {
        Ok(norm2(value))
    }
}

/// Extends a decimal to two decimals when it carries fewer, the XSD
/// amount types reading at least two decimals.
fn norm2(mut value: Decimal) -> Decimal {
    if value.scale() < 2 {
        value.rescale(2);
    }
    value
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing case"
)]
mod tests {
    use super::*;
    use crate::Document;
    use crate::ids::{PartitaIva, Provincia, UnitaMisura};
    use crate::party::Address;
    use iso_country::Country;

    /// A synthetic check-valid `partita IVA`, never a real company.
    fn alpha_piva() -> PartitaIva {
        "00012345674"
            .parse()
            .expect("a synthetic valid partita IVA")
    }

    /// A synthetic check-valid `partita IVA`, never a real company.
    fn beta_piva() -> PartitaIva {
        "00076543214"
            .parse()
            .expect("a synthetic valid partita IVA")
    }

    /// The domestic buyer of the Rome seat.
    fn buyer_rome() -> Buyer {
        Buyer::denomination(
            "BETA SRL".parse().expect("a valid denomination"),
            beta_piva(),
            rome(),
        )
    }

    /// The domestic recipient code of the buyer.
    fn codice() -> Recipient {
        Recipient::Codice(CodiceDestinatario::try_new("ABC1234").expect("a valid code"))
    }

    /// The non-taxable line with the export nature and the reference.
    fn not_taxed_line(reference: &str) -> Line {
        Line::builder()
            .description("MERCE".parse().expect("a valid description"))
            .quantity(Amount(Decimal::new(2, 2)))
            .unit_price(Amount(Decimal::new(10000, 2)))
            .vat(VatTreatment::NotTaxed {
                natura: Natura::N3_2,
                riferimento_normativo: reference.parse().expect("a valid reference"),
            })
            .build()
    }

    fn rome() -> Address {
        Address::italian(
            "VIA ROMA 1".parse().expect("a valid address"),
            "00100".parse().expect("a valid CAP"),
            "ROMA".parse().expect("a valid comune"),
            Provincia::Rm,
        )
    }

    fn seller() -> Seller {
        Seller::denomination(
            "ALPHA SRL".parse().expect("a valid denomination"),
            alpha_piva(),
            rome(),
            RegimeFiscale::Rf01,
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

    fn supplier_de() -> Business {
        Business::eu(
            crate::ids::EuCountry::De,
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
            beta_piva(),
            Address::italian(
                "VIA MILANO 2".parse().expect("a valid address"),
                "20100".parse().expect("a valid CAP"),
                "MILANO".parse().expect("a valid comune"),
                Provincia::Mi,
            ),
        )
    }

    fn seven_ter_line() -> SevenTerLine {
        SevenTerLine::builder()
            .description(
                "SERVIZIO DI CONSULENZA"
                    .parse()
                    .expect("a valid description"),
            )
            .quantity(Amount(Decimal::new(10000, 2)))
            .unit_price(Amount(Decimal::new(15000, 2)))
            .build()
    }

    fn taxed_line(value: i64) -> Line {
        Line::builder()
            .description("SERVIZIO".parse().expect("a valid description"))
            .quantity(Amount(Decimal::new(100, 2)))
            .unit_price(Amount(Decimal::new(value, 2)))
            .vat(VatTreatment::Taxed(
                "22.00".parse::<Aliquota>().expect("a valid rate"),
            ))
            .build()
    }

    fn progressivo() -> ProgressivoInvio {
        "00001".parse().expect("a valid progressivo")
    }

    fn date() -> Date {
        "2026-11-04".parse().expect("a valid date")
    }

    fn numero() -> Numero {
        "00012".parse().expect("a valid number")
    }

    #[test]
    fn cross_border_builds_and_passes_validate() {
        let doc = Invoice::cross_border_services(&seller(), &buyer_us(), seven_ter_line())
            .progressivo(progressivo())
            .date(date())
            .numero(numero())
            .causali(vec![
                "CONSULENZA AI SENSI DELL'ART. 7-TER DPR 633/72"
                    .parse()
                    .expect("a valid causale"),
            ])
            .build()
            .expect("the invoice builds");
        assert_eq!(validate::validate(&doc), Vec::new());
    }

    #[test]
    fn cross_border_lines_carry_the_seven_ter_nature() {
        let doc = Invoice::cross_border_services(&seller(), &buyer_us(), seven_ter_line())
            .progressivo(progressivo())
            .date(date())
            .numero(numero())
            .build()
            .expect("the invoice builds");
        let body = &doc.content.fattura_elettronica_body[0].content;
        let line = &body.dati_beni_servizi.content.dettaglio_linee[0].content;
        assert_eq!(line.natura, Some(crate::generated::fpr12::NaturaType::N21));
        assert_eq!(line.aliquota_iva.0, Decimal::new(0, 2));
        let riep = &body.dati_beni_servizi.content.dati_riepilogo[0].content;
        assert_eq!(
            riep.riferimento_normativo.as_deref(),
            Some("ART. 7-TER DPR 633/72")
        );
    }

    #[test]
    fn the_pec_recipient_writes_the_generic_code() {
        let doc = Invoice::domestic(
            &seller(),
            &Buyer::denomination(
                "BETA SRL".parse().expect("a valid denomination"),
                beta_piva(),
                Address::italian(
                    "VIA MILANO 2".parse().expect("a valid address"),
                    "20100".parse().expect("a valid CAP"),
                    "MILANO".parse().expect("a valid comune"),
                    Provincia::Mi,
                ),
            ),
            Recipient::Pec("beta@pec.it".parse().expect("a valid pec")),
            taxed_line(10000),
        )
        .progressivo(progressivo())
        .date(date())
        .numero(numero())
        .build()
        .expect("the invoice builds");
        let transmission = doc
            .content
            .fattura_elettronica_header
            .content
            .dati_trasmissione
            .content
            .clone();
        assert_eq!(transmission.codice_destinatario.as_str(), "0000000");
        assert_eq!(
            transmission.pec_destinatario.as_deref(),
            Some("beta@pec.it")
        );
        assert_eq!(doc.versione, FormatoTrasmissioneType::Fpr12);
    }

    #[test]
    fn the_self_invoice_fills_the_payment() {
        let doc = SelfInvoice::td17(
            &supplier_de(),
            &buyer_company(),
            Recipient::Codice(CodiceDestinatario::try_new("ABC1234").expect("a valid code")),
            taxed_line(10000),
        )
        .progressivo(progressivo())
        .date("2026-11-05".parse().expect("a valid date"))
        .numero("001".parse().expect("a valid number"))
        .causali(vec!["AUTOFATTURA".parse().expect("a valid causale")])
        .build()
        .expect("the self-invoice builds");
        assert_eq!(validate::validate(&doc), Vec::new());
        let pagamento = &doc
            .content
            .fattura_elettronica_body
            .first()
            .expect("one body")
            .content
            .dati_pagamento;
        assert_eq!(
            pagamento
                .first()
                .expect("a payment")
                .content
                .condizioni_pagamento,
            CondizioniPagamentoType::Tp01
        );
        let seller_regime = doc
            .content
            .fattura_elettronica_header
            .content
            .cedente_prestatore
            .content
            .dati_anagrafici
            .content
            .regime_fiscale;
        assert_eq!(
            seller_regime,
            crate::generated::fpr12::RegimeFiscaleType::Rf01
        );
    }

    #[test]
    fn the_sconto_validates_its_value() {
        assert!(Sconto::discount_percent(Decimal::ONE_HUNDRED).is_ok());
        assert!(Sconto::discount_percent(Decimal::ZERO).is_err());
        assert!(Sconto::markup_amount(Decimal::new(-100, 2)).is_err());
        assert!(Sconto::discount_amount(Decimal::new(123_456_789, 9)).is_err());
        // The `Percentuale` fits the `RateType` of at most two decimals.
        assert!(Sconto::discount_percent(Decimal::new(1234, 4)).is_err());
        // The `Importo` fits the nine integer digits of the XSD.
        assert!(Sconto::discount_amount(Decimal::from(1_000_000_000)).is_err());
    }

    #[test]
    fn the_line_amount_checks_the_xsd_bounds() {
        let too_large =
            |value: Decimal, field: AmountField| amount_in_range(value, field, 1).is_err();
        assert!(too_large(
            Decimal::from(1_000_000_000),
            AmountField::UnitPrice
        ));
        assert!(!too_large(
            Decimal::from(999_999_999),
            AmountField::UnitPrice
        ));
        assert!(too_large(
            Decimal::new(1_000_000_000_000, 0),
            AmountField::Quantity
        ));
        // The `QuantitaType` is non-negative.
        assert!(too_large(Decimal::new(-100, 2), AmountField::Quantity));
        // A negative `PrezzoUnitario` is allowed by the `Amount8DecimalType`.
        assert!(!too_large(Decimal::new(-100, 2), AmountField::UnitPrice));
    }

    #[test]
    fn the_foreign_currency_writes_euros_and_the_management_data() {
        let currency = InvoiceCurrency::Foreign {
            currency: Currency::USD,
            rate: "1.08".parse().expect("a valid rate"),
            rate_date: "2026-11-04".parse().expect("a valid date"),
        };
        let doc = Invoice::cross_border_services(&seller(), &buyer_us(), seven_ter_line())
            .progressivo(progressivo())
            .date(date())
            .numero("00011".parse().expect("a valid number"))
            .currency(currency)
            .build()
            .expect("the invoice builds");
        let body = &doc.content.fattura_elettronica_body[0].content;
        let line = &body.dati_beni_servizi.content.dettaglio_linee[0].content;
        // 150 USD at 1.08 USD per euro, the euro value is 138.89.
        assert_eq!(line.prezzo_unitario.0, Decimal::new(13889, 2));
        assert_eq!(line.altri_dati_gestionali.len(), 2);
        assert_eq!(line.altri_dati_gestionali[0].content.tipo_dato, "USD AMT");
        assert_eq!(line.altri_dati_gestionali[1].content.tipo_dato, "USD RATE");
        assert_eq!(
            body.dati_generali
                .content
                .dati_generali_documento
                .content
                .divisa,
            "EUR"
        );
        let totale = body
            .dati_generali
            .content
            .dati_generali_documento
            .content
            .importo_totale_documento
            .expect("the total is present");
        // 138.89 euros per unit, one hundred units.
        assert_eq!(totale.0, Decimal::new(1_388_900, 2));
    }

    #[test]
    fn the_line_numbers_follow_the_document_order() {
        let doc = Invoice::domestic(
            &seller(),
            &Buyer::denomination(
                "BETA SRL".parse().expect("a valid denomination"),
                beta_piva(),
                Address::italian(
                    "VIA MILANO 2".parse().expect("a valid address"),
                    "20100".parse().expect("a valid CAP"),
                    "MILANO".parse().expect("a valid comune"),
                    Provincia::Mi,
                ),
            ),
            Recipient::Codice(CodiceDestinatario::try_new("ABC1234").expect("a valid code")),
            taxed_line(100),
        )
        .progressivo(progressivo())
        .date("2026-11-05".parse().expect("a valid date"))
        .numero("001".parse().expect("a valid number"))
        .lines(vec![taxed_line(50)])
        .build()
        .expect("the invoice builds");
        let lines = &doc.content.fattura_elettronica_body[0]
            .content
            .dati_beni_servizi
            .content
            .dettaglio_linee;
        assert_eq!(lines[0].content.numero_linea, 1);
        assert_eq!(lines[1].content.numero_linea, 2);
        assert_eq!(validate::validate(&doc), Vec::new());
    }

    #[test]
    fn the_unit_of_measure_is_carried() {
        let line = Line::builder()
            .description("BENE".parse().expect("a valid description"))
            .quantity(Amount(Decimal::new(100, 2)))
            .unit_of_measure(UnitaMisura::try_new("KG").expect("a valid unit of measure"))
            .unit_price(Amount(Decimal::new(1000, 2)))
            .vat(VatTreatment::Taxed(
                "22.00".parse::<Aliquota>().expect("a valid rate"),
            ))
            .build();
        let doc = Invoice::domestic(
            &seller(),
            &Buyer::denomination(
                "BETA SRL".parse().expect("a valid denomination"),
                beta_piva(),
                rome(),
            ),
            Recipient::Codice(CodiceDestinatario::try_new("ABC1234").expect("a valid code")),
            line,
        )
        .progressivo(progressivo())
        .date(date())
        .numero(numero())
        .build()
        .expect("the invoice builds");
        let line = &doc.content.fattura_elettronica_body[0]
            .content
            .dati_beni_servizi
            .content
            .dettaglio_linee[0]
            .content;
        assert_eq!(line.unita_misura.as_deref(), Some("KG"));
    }

    #[test]
    fn the_sole_trader_seller_carries_the_codice_fiscale() {
        let seller = Seller::sole_trader(
            "Mario".parse().expect("a valid name"),
            "Rossi".parse().expect("a valid name"),
            alpha_piva(),
            "MRALNE80E05H501C".parse().expect("a valid codice fiscale"),
            rome(),
            RegimeFiscale::Rf19,
        );
        let cedente = seller
            .party()
            .cedente(*seller.regime())
            .expect("the seller is complete");
        assert_eq!(
            cedente
                .content
                .dati_anagrafici
                .content
                .codice_fiscale
                .as_deref(),
            Some("MRALNE80E05H501C")
        );
        assert_eq!(
            cedente.content.dati_anagrafici.content.regime_fiscale,
            crate::generated::fpr12::RegimeFiscaleType::Rf19
        );
    }

    /// The `BuildError::Amount` message names the line and the field the
    /// consumer sees on a refused amount.
    #[test]
    fn the_amount_error_names_the_line_and_field() {
        let unit_price = BuildError::Amount {
            line: 2,
            field: AmountField::UnitPrice,
            value: Decimal::new(1_000_000_000, 0),
        };
        assert_eq!(
            unit_price.to_string(),
            "line 2 the unit price is not a valid amount: 1000000000"
        );
        let quantity = BuildError::Amount {
            line: 3,
            field: AmountField::Quantity,
            value: Decimal::new(-100, 2),
        };
        assert_eq!(
            quantity.to_string(),
            "line 3 the quantity is not a valid amount: -1.00"
        );
    }

    /// The `Line` accessor reads the nature of the non-taxable treatment
    /// and is absent for a taxed line.
    #[test]
    fn the_line_natura_reads_the_treatment() {
        assert!(taxed_line(10000).natura().is_none());
        let line = not_taxed_line("ESPORTAZIONE EFFETTIVA");
        assert_eq!(line.natura(), Some(&Natura::N3_2));
    }

    /// The riepilogo group of the nature carries the reference of the
    /// last line of the group, the one the document sends.
    #[test]
    fn the_riepilogo_carries_the_last_reference_of_the_nature() {
        let doc = Invoice::domestic(
            &seller(),
            &buyer_rome(),
            codice(),
            not_taxed_line("PRIMO RIFERIMENTO"),
        )
        .progressivo(progressivo())
        .date(date())
        .numero(numero())
        .lines(vec![not_taxed_line("SECONDO RIFERIMENTO")])
        .build()
        .expect("the invoice builds");
        let body = &doc.content.fattura_elettronica_body[0].content;
        let riep = &body.dati_beni_servizi.content.dati_riepilogo[0].content;
        assert_eq!(riep.natura, Some(crate::generated::fpr12::NaturaType::N32));
        assert_eq!(
            riep.riferimento_normativo.as_deref(),
            Some("SECONDO RIFERIMENTO")
        );
        assert_eq!(validate::validate(&doc), Vec::new());
    }

    /// The `PrezzoUnitario` of eight decimals fits the XSD pattern and
    /// is written at its full precision.
    #[test]
    fn the_eight_decimal_amount_fits_the_xsd_types() {
        let line = Line::builder()
            .description("SERVIZIO".parse().expect("a valid description"))
            .quantity(Amount(Decimal::new(100, 2)))
            .unit_price(Amount(Decimal::new(100_000_001, 8)))
            .vat(VatTreatment::Taxed(
                "22.00".parse::<Aliquota>().expect("a valid rate"),
            ))
            .build();
        let doc = Invoice::domestic(&seller(), &buyer_rome(), codice(), line)
            .progressivo(progressivo())
            .date(date())
            .numero(numero())
            .build()
            .expect("the invoice builds");
        let line = &doc.content.fattura_elettronica_body[0]
            .content
            .dati_beni_servizi
            .content
            .dettaglio_linee[0]
            .content;
        assert_eq!(line.prezzo_unitario.0.scale(), 8);
        assert_eq!(line.prezzo_unitario.0, Decimal::new(100_000_001, 8));
        let xml = doc.to_xml_string().expect("the invoice serialises");
        assert!(xml.contains("<PrezzoUnitario>1.00000001</PrezzoUnitario>"));
    }

    /// The `PrezzoUnitario` of nine decimals does not fit the XSD
    /// pattern and the build refuses it.
    #[test]
    fn the_nine_decimal_amount_fails_the_build() {
        let line = Line::builder()
            .description("SERVIZIO".parse().expect("a valid description"))
            .quantity(Amount(Decimal::new(100, 2)))
            .unit_price(Amount(Decimal::new(100_000_000_001, 9)))
            .vat(VatTreatment::Taxed(
                "22.00".parse::<Aliquota>().expect("a valid rate"),
            ))
            .build();
        let error = Invoice::domestic(&seller(), &buyer_rome(), codice(), line)
            .progressivo(progressivo())
            .date(date())
            .numero(numero())
            .build()
            .expect_err("nine decimals do not fit the XSD pattern");
        assert_eq!(
            error,
            BuildError::Amount {
                line: 1,
                field: AmountField::UnitPrice,
                value: Decimal::new(100_000_000_001, 9),
            }
        );
    }

    /// The amounts with fewer than two decimals are normalized to the
    /// two the XSD pattern requires.
    #[test]
    fn the_sub_two_decimal_amounts_write_two_decimals() {
        let line = Line::builder()
            .description("SERVIZIO".parse().expect("a valid description"))
            .quantity(Amount(Decimal::new(2, 0)))
            .unit_price(Amount(Decimal::new(55, 1)))
            .vat(VatTreatment::Taxed(
                "22.00".parse::<Aliquota>().expect("a valid rate"),
            ))
            .build();
        let doc = Invoice::domestic(&seller(), &buyer_rome(), codice(), line)
            .progressivo(progressivo())
            .date(date())
            .numero(numero())
            .build()
            .expect("the invoice builds");
        let line = &doc.content.fattura_elettronica_body[0]
            .content
            .dati_beni_servizi
            .content
            .dettaglio_linee[0]
            .content;
        assert_eq!(
            line.quantita
                .as_ref()
                .expect("the quantity is present")
                .0
                .scale(),
            2
        );
        assert_eq!(line.prezzo_unitario.0.scale(), 2);
        let xml = doc.to_xml_string().expect("the invoice serialises");
        assert!(xml.contains("<PrezzoUnitario>5.50</PrezzoUnitario>"));
        assert!(xml.contains("<Quantita>2.00</Quantita>"));
    }
}
