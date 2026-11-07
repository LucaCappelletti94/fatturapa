//! The typed construction of the `FatturaElettronica` document, on top of
//! the generated types.
//!
//! The scenarios that the specification dictates are named constructors,
//! `Invoice::domestic`, `Invoice::cross_border_services` and
//! `SelfInvoice::td17` and `td19`, each with a typestate builder. The
//! scenario parties and the first line are parameters of the constructor,
//! and the `ProgressivoInvio`, the `Data` and the `Numero` are required
//! setters, so a document missing a required field does not compile.
//!
//! Every setter accepts any value convertible into the typed one through
//! `TryFrom`, its error convertible into `IdError`, and never fails. The
//! first conversion error is kept in the builder and returned by `build`
//! as `BuildError::Id` before any other check, and once it is kept the
//! later setters skip their conversion and only advance the typestate.
//!
//! Every `build` checks the scenario rules, computes the line totals, the
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
    Aliquota, Causale, CodiceDestinatario, Description, ExchangeRate, IdDocumento, IdError, Natura,
    Numero, Pec, ProgressivoInvio, RegimeFiscale, RiferimentoNormativo, UnitaMisura,
};
use crate::party::{Business, Buyer, Company, Party, Seller};
use crate::validate::{self, Violation};
use iso_currency::Currency;
use rust_decimal::Decimal;
use std::marker::PhantomData;
/// The typestate markers of the builders, the type parameter of each
/// required field, the `Unset` marker until its setter was called and the
/// `Set` marker after.
#[doc(hidden)]
pub mod typestate {
    /// The required field of a builder that has not been set yet.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Unset;

    /// The required field of a builder that has been set.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Set;
}

/// The value a required setter stored, the typestate of the builder
/// guaranteeing its presence.
#[expect(
    clippy::expect_used,
    reason = "the typestate of the builder guarantees the field is set"
)]
const fn required<T>(slot: Option<T>) -> T {
    slot.expect("the typestate of the builder guarantees the field is set")
}

/// The conversion of a builder setter, shared by the setters of every
/// builder.
///
/// When no error is stored yet, the value is converted into `T` and the
/// converted value is returned for the caller to store. When an error is
/// already stored, the conversion is skipped and `None` is returned. When
/// the conversion fails, the failure is kept as the first error of the
/// builder and `None` is returned.
fn convert<T, G>(error: &mut Option<BuildError>, field: &'static str, value: G) -> Option<T>
where
    T: TryFrom<G>,
    <T as TryFrom<G>>::Error: Into<IdError>,
{
    if error.is_some() {
        return None;
    }
    match T::try_from(value) {
        Ok(value) => Some(value),
        Err(cause) => {
            *error = Some(BuildError::Id {
                field,
                error: cause.into(),
            });
            None
        }
    }
}

/// The `CodiceDestinatario` the `SDI` assigns to a non-resident buyer.
const FOREIGN_CODICE_DESTINATARIO: &str = "XXXXXXX";

/// The `CodiceDestinatario` written with a `PECDestinatario` or an unknown
/// channel.
const PEC_CODICE_DESTINATARIO: &str = "0000000";

/// The 77.47 EUR stamp duty threshold of the DPR 642/72, the `DatiBollo`
/// block written only when the total exceeds it.
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
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
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
    /// The first conversion error of a builder setter, with the name of
    /// the setter it came from, so the caller sees which input failed
    /// without re-checking them.
    #[error("the {field} is not valid: {error}")]
    Id {
        /// The name of the setter that produced the error.
        field: &'static str,
        /// The conversion error.
        error: IdError,
    },
    /// The assembled document fails one or more `SDI` checks.
    #[error("the document fails {} SDI check(s)", .0.len())]
    Violations(Vec<Violation>),
}

/// The amount field of a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl Recipient {
    /// The `CodiceDestinatario` channel, converted from `value`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the value is not a `CodiceDestinatario`.
    pub fn try_codice<G>(codice: G) -> Result<Self, IdError>
    where
        CodiceDestinatario: TryFrom<G>,
        <CodiceDestinatario as TryFrom<G>>::Error: Into<IdError>,
    {
        Ok(Self::Codice(
            CodiceDestinatario::try_from(codice).map_err(Into::into)?,
        ))
    }

    /// The `PECDestinatario` channel, converted from `value`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the value is not a `PEC` address.
    pub fn try_pec<G>(pec: G) -> Result<Self, IdError>
    where
        Pec: TryFrom<G>,
        <Pec as TryFrom<G>>::Error: Into<IdError>,
    {
        Ok(Self::Pec(Pec::try_from(pec).map_err(Into::into)?))
    }
}

/// The `AliquotaIVA` and the `Natura` of a line, one of the two, so a
/// taxed nature or a non-taxed rate with a percentage cannot be written.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// A taxable line, its `AliquotaIVA` converted from `value`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the value is not a positive percentage of at
    /// most 100 with at most two decimals.
    pub fn taxed<G>(aliquota: G) -> Result<Self, IdError>
    where
        Aliquota: TryFrom<G>,
        <Aliquota as TryFrom<G>>::Error: Into<IdError>,
    {
        Ok(Self::Taxed(
            Aliquota::try_from(aliquota).map_err(Into::into)?,
        ))
    }

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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// A document priced in a foreign currency, its `Divisa` resolved to
    /// the `Currency` enum, the `rate` and its `date` converted.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the `rate` or the `date` is not valid.
    pub fn try_foreign<R, D>(currency: Currency, rate: R, rate_date: D) -> Result<Self, IdError>
    where
        ExchangeRate: TryFrom<R>,
        <ExchangeRate as TryFrom<R>>::Error: Into<IdError>,
        Date: TryFrom<D>,
        <Date as TryFrom<D>>::Error: Into<IdError>,
    {
        Ok(Self::Foreign {
            currency,
            rate: ExchangeRate::try_from(rate).map_err(Into::into)?,
            rate_date: Date::try_from(rate_date).map_err(Into::into)?,
        })
    }
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Sconto {
    /// Whether the block is the `SC` discount or the `MG` markup.
    discount: bool,
    /// Whether the value is the `Percentuale` or the `Importo`.
    percent: bool,
    /// The value, at most eight decimals.
    value: Decimal,
}

impl Sconto {
    /// A discount of `value` percent, `SC` and `Percentuale`, converted
    /// from `value`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the value is not a positive percentage of at
    /// most 100 with at most two decimals.
    pub fn discount_percent<G>(value: G) -> Result<Self, IdError>
    where
        Decimal: TryFrom<G>,
        <Decimal as TryFrom<G>>::Error: Into<IdError>,
    {
        Self::new(true, true, value)
    }

    /// A markup of `value` percent, `MG` and `Percentuale`, converted
    /// from `value`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the value is not a positive percentage of at
    /// most 100 with at most two decimals.
    pub fn markup_percent<G>(value: G) -> Result<Self, IdError>
    where
        Decimal: TryFrom<G>,
        <Decimal as TryFrom<G>>::Error: Into<IdError>,
    {
        Self::new(false, true, value)
    }

    /// A discount of `value` euros, `SC` and `Importo`, converted from
    /// `value`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the value is not a positive amount of the
    /// `Amount8DecimalType` bounds.
    pub fn discount_amount<G>(value: G) -> Result<Self, IdError>
    where
        Decimal: TryFrom<G>,
        <Decimal as TryFrom<G>>::Error: Into<IdError>,
    {
        Self::new(true, false, value)
    }

    /// A markup of `value` euros, `MG` and `Importo`, converted from
    /// `value`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the value is not a positive amount of the
    /// `Amount8DecimalType` bounds.
    pub fn markup_amount<G>(value: G) -> Result<Self, IdError>
    where
        Decimal: TryFrom<G>,
        <Decimal as TryFrom<G>>::Error: Into<IdError>,
    {
        Self::new(false, false, value)
    }

    /// Validates the value and builds the block. The `Percentuale` fits
    /// the `RateType` of at most two decimals, the `Importo` the
    /// `Amount8DecimalType` of at most eight decimals and nine integer
    /// digits.
    fn new<G>(discount: bool, percent: bool, value: G) -> Result<Self, IdError>
    where
        Decimal: TryFrom<G>,
        <Decimal as TryFrom<G>>::Error: Into<IdError>,
    {
        let original = Decimal::try_from(value).map_err(Into::into)?;
        let mut value = original;
        let max_scale = if percent { 2 } else { 8 };
        value.rescale(max_scale);
        if value != original
            || value <= Decimal::ZERO
            || (percent && value > Decimal::ONE_HUNDRED)
            || (!percent && value >= unit_price_bound())
        {
            return Err(IdError::Sconto(original.to_string()));
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

/// The serialised form of a [`Sconto`], the kind flags and the value as a
/// decimal string, re-validated on the way in.
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
struct ScontoForm {
    /// Whether the block is the `SC` discount.
    discount: bool,
    /// Whether the value is the `Percentuale`.
    percent: bool,
    /// The value, as a decimal string.
    value: String,
}

#[cfg(feature = "serde")]
impl serde::Serialize for Sconto {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ScontoForm {
            discount: self.discount,
            percent: self.percent,
            value: self.value.to_string(),
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Sconto {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ScontoForm {
            discount,
            percent,
            value,
        } = ScontoForm::deserialize(deserializer)?;
        match (discount, percent) {
            (true, true) => Self::discount_percent(value.as_str()),
            (false, true) => Self::markup_percent(value.as_str()),
            (true, false) => Self::discount_amount(value.as_str()),
            (false, false) => Self::markup_amount(value.as_str()),
        }
        .map_err(serde::de::Error::custom)
    }
}

/// One `DettaglioLinee` of a typed invoice.
///
/// The `NumeroLinea` is assigned by the invoice at `build`, in document
/// order. The `PrezzoUnitario` is in the currency the invoice carries, and
/// the `PrezzoTotale` is computed by the invoice from the unit price, the
/// quantity and the `ScontoMaggiorazione` blocks, per the 00423 rule.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    sconti: Vec<Sconto>,
    /// The `AliquotaIVA` or the `Natura` of the line.
    vat: VatTreatment,
}

/// The optional fields of a document line, embedded by the
/// [`LineBuilder`] and the [`SevenTerLineBuilder`], with their shared
/// setters and the first conversion error of the builders.
#[derive(Clone, Debug, PartialEq, Eq)]
struct LineOptionals {
    /// The `Quantita`, absent when the line is not quantity-priced and the
    /// total reads the unit price.
    quantity: Option<Amount>,
    /// The `UnitaMisura`.
    unit_of_measure: Option<UnitaMisura>,
    /// The `ScontoMaggiorazione` blocks, in document order.
    sconti: Option<Vec<Sconto>>,
    /// The first conversion error of the setters, if any.
    error: Option<BuildError>,
}

impl LineOptionals {
    /// The empty fields, no value set and no error kept.
    const fn new() -> Self {
        Self {
            quantity: None,
            unit_of_measure: None,
            sconti: None,
            error: None,
        }
    }

    /// The `Quantita`, converted from `value`.
    pub(crate) fn quantity<G>(&mut self, value: G)
    where
        Amount: TryFrom<G>,
        <Amount as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(quantity) = convert(&mut self.error, "quantity", value) {
            self.quantity = Some(quantity);
        }
    }

    /// The `Quantita`, converted from `value`, absent when `value` is
    /// `None`.
    pub(crate) fn maybe_quantity<G>(&mut self, value: Option<G>)
    where
        Amount: TryFrom<G>,
        <Amount as TryFrom<G>>::Error: Into<IdError>,
    {
        match value {
            Some(value) => {
                if let Some(quantity) = convert(&mut self.error, "quantity", value) {
                    self.quantity = Some(quantity);
                }
            }
            None => self.quantity = None,
        }
    }

    /// The `UnitaMisura`, converted from `value`.
    pub(crate) fn unit_of_measure<G>(&mut self, value: G)
    where
        UnitaMisura: TryFrom<G>,
        <UnitaMisura as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(unit_of_measure) = convert(&mut self.error, "unit_of_measure", value) {
            self.unit_of_measure = Some(unit_of_measure);
        }
    }

    /// The `UnitaMisura`, converted from `value`, absent when `value` is
    /// `None`.
    pub(crate) fn maybe_unit_of_measure<G>(&mut self, value: Option<G>)
    where
        UnitaMisura: TryFrom<G>,
        <UnitaMisura as TryFrom<G>>::Error: Into<IdError>,
    {
        match value {
            Some(value) => {
                if let Some(unit_of_measure) = convert(&mut self.error, "unit_of_measure", value) {
                    self.unit_of_measure = Some(unit_of_measure);
                }
            }
            None => self.unit_of_measure = None,
        }
    }

    /// The `ScontoMaggiorazione` blocks, in document order.
    pub(crate) fn sconti<G>(&mut self, value: G)
    where
        Vec<Sconto>: TryFrom<G>,
        <Vec<Sconto> as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(sconti) = convert(&mut self.error, "sconti", value) {
            self.sconti = Some(sconti);
        }
    }
}

/// The builder of a [`Line`], which checks the required fields at compile
/// time and defers the first conversion error of the setters to `build`.
///
/// The `description`, the `unit_price` and the `vat` are required, and the
/// type parameters `D`, `P` and `V` carry the marker of each one, the
/// `Set` marker once its setter was called, so a line missing a required
/// field does not compile and the `build` is available only when every
/// required field is set. The optional fields, the `quantity`, the
/// `unit_of_measure` and the `sconti`, carry plain slots with their
/// setters available in every state.
#[must_use = "the builder does nothing until you call `build()` on it to finish building"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineBuilder<D = typestate::Unset, P = typestate::Unset, V = typestate::Unset> {
    /// The `Descrizione`.
    description: Option<Description>,
    /// The optional fields, their shared setters and the first conversion
    /// error of the builders.
    optional: LineOptionals,
    /// The `PrezzoUnitario`, in the currency the invoice carries.
    unit_price: Option<Amount>,
    /// The `AliquotaIVA` or the `Natura` of the line.
    vat: Option<VatTreatment>,
    /// The typestate the builder has reached.
    _state: PhantomData<(D, P, V)>,
}

impl<D, P, V> LineBuilder<D, P, V> {
    /// The builder moved into the typestate the setter advanced to.
    fn advance<ND, NP, NV>(self) -> LineBuilder<ND, NP, NV> {
        let Self {
            description,
            optional,
            unit_price,
            vat,
            ..
        } = self;
        LineBuilder {
            description,
            optional,
            unit_price,
            vat,
            _state: PhantomData,
        }
    }

    /// The `Quantita`, converted from `value`.
    pub fn quantity<G>(mut self, value: G) -> Self
    where
        Amount: TryFrom<G>,
        <Amount as TryFrom<G>>::Error: Into<IdError>,
    {
        self.optional.quantity(value);
        self
    }

    /// The `Quantita`, converted from `value`, absent when `value` is
    /// `None`.
    pub fn maybe_quantity<G>(mut self, value: Option<G>) -> Self
    where
        Amount: TryFrom<G>,
        <Amount as TryFrom<G>>::Error: Into<IdError>,
    {
        self.optional.maybe_quantity(value);
        self
    }

    /// The `UnitaMisura`, converted from `value`.
    pub fn unit_of_measure<G>(mut self, value: G) -> Self
    where
        UnitaMisura: TryFrom<G>,
        <UnitaMisura as TryFrom<G>>::Error: Into<IdError>,
    {
        self.optional.unit_of_measure(value);
        self
    }

    /// The `UnitaMisura`, converted from `value`, absent when `value` is
    /// `None`.
    pub fn maybe_unit_of_measure<G>(mut self, value: Option<G>) -> Self
    where
        UnitaMisura: TryFrom<G>,
        <UnitaMisura as TryFrom<G>>::Error: Into<IdError>,
    {
        self.optional.maybe_unit_of_measure(value);
        self
    }

    /// The `ScontoMaggiorazione` blocks, in document order.
    pub fn sconti<G>(mut self, value: G) -> Self
    where
        Vec<Sconto>: TryFrom<G>,
        <Vec<Sconto> as TryFrom<G>>::Error: Into<IdError>,
    {
        self.optional.sconti(value);
        self
    }
}

impl<P, V> LineBuilder<typestate::Unset, P, V> {
    /// The `Descrizione`, converted from `value`.
    pub fn description<G>(mut self, value: G) -> LineBuilder<typestate::Set, P, V>
    where
        Description: TryFrom<G>,
        <Description as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(description) = convert(&mut self.optional.error, "description", value) {
            self.description = Some(description);
        }
        self.advance()
    }
}

impl<D, V> LineBuilder<D, typestate::Unset, V> {
    /// The `PrezzoUnitario`, in the currency the invoice carries,
    /// converted from `value`.
    pub fn unit_price<G>(mut self, value: G) -> LineBuilder<D, typestate::Set, V>
    where
        Amount: TryFrom<G>,
        <Amount as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(unit_price) = convert(&mut self.optional.error, "unit_price", value) {
            self.unit_price = Some(unit_price);
        }
        self.advance()
    }
}

impl<D, P> LineBuilder<D, P, typestate::Unset> {
    /// The `AliquotaIVA` or the `Natura` of the line.
    pub fn vat<G>(mut self, value: G) -> LineBuilder<D, P, typestate::Set>
    where
        VatTreatment: TryFrom<G>,
        <VatTreatment as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(vat) = convert(&mut self.optional.error, "vat", value) {
            self.vat = Some(vat);
        }
        self.advance()
    }
}

impl LineBuilder<typestate::Set, typestate::Set, typestate::Set> {
    /// Finishes building the line.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the setters.
    ///
    /// # Panics
    ///
    /// Panics if a required field is unset, which the typestate of the
    /// setter chain prevents.
    pub fn build(self) -> Result<Line, BuildError> {
        if let Some(error) = self.optional.error {
            return Err(error);
        }
        Ok(Line {
            description: required(self.description),
            quantity: self.optional.quantity,
            unit_of_measure: self.optional.unit_of_measure,
            unit_price: required(self.unit_price),
            sconti: self.optional.sconti.unwrap_or_default(),
            vat: required(self.vat),
        })
    }
}

impl Line {
    /// The builder of a [`Line`], which checks the required fields at
    /// compile time.
    pub const fn builder() -> LineBuilder {
        LineBuilder {
            description: None,
            optional: LineOptionals::new(),
            unit_price: None,
            vat: None,
            _state: PhantomData,
        }
    }

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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    sconti: Vec<Sconto>,
}

/// The builder of a [`SevenTerLine`], which checks the required fields at
/// compile time and defers the first conversion error of the setters to
/// `build`.
///
/// The `description` and the `unit_price` are required, and the type
/// parameters `D` and `P` carry the marker of each one, the `Set` marker
/// once its setter was called, so a line missing a required field does
/// not compile and the `build` is available only when every required
/// field is set. The scenario writes the `N2.1` nature and the reference
/// on every line, so the builder carries no `vat` setter. The optional
/// fields, the `quantity`, the `unit_of_measure` and the `sconti`, carry
/// plain slots with their setters available in every state.
#[must_use = "the builder does nothing until you call `build()` on it to finish building"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SevenTerLineBuilder<D = typestate::Unset, P = typestate::Unset> {
    /// The `Descrizione`.
    description: Option<Description>,
    /// The optional fields, their shared setters and the first conversion
    /// error of the builders.
    optional: LineOptionals,
    /// The `PrezzoUnitario`, in the currency the invoice carries.
    unit_price: Option<Amount>,
    /// The typestate the builder has reached.
    _state: PhantomData<(D, P)>,
}

impl<D, P> SevenTerLineBuilder<D, P> {
    /// The builder moved into the typestate the setter advanced to.
    fn advance<ND, NP>(self) -> SevenTerLineBuilder<ND, NP> {
        let Self {
            description,
            optional,
            unit_price,
            ..
        } = self;
        SevenTerLineBuilder {
            description,
            optional,
            unit_price,
            _state: PhantomData,
        }
    }

    /// The `Quantita`, converted from `value`.
    pub fn quantity<G>(mut self, value: G) -> Self
    where
        Amount: TryFrom<G>,
        <Amount as TryFrom<G>>::Error: Into<IdError>,
    {
        self.optional.quantity(value);
        self
    }

    /// The `Quantita`, converted from `value`, absent when `value` is
    /// `None`.
    pub fn maybe_quantity<G>(mut self, value: Option<G>) -> Self
    where
        Amount: TryFrom<G>,
        <Amount as TryFrom<G>>::Error: Into<IdError>,
    {
        self.optional.maybe_quantity(value);
        self
    }

    /// The `UnitaMisura`, converted from `value`.
    pub fn unit_of_measure<G>(mut self, value: G) -> Self
    where
        UnitaMisura: TryFrom<G>,
        <UnitaMisura as TryFrom<G>>::Error: Into<IdError>,
    {
        self.optional.unit_of_measure(value);
        self
    }

    /// The `UnitaMisura`, converted from `value`, absent when `value` is
    /// `None`.
    pub fn maybe_unit_of_measure<G>(mut self, value: Option<G>) -> Self
    where
        UnitaMisura: TryFrom<G>,
        <UnitaMisura as TryFrom<G>>::Error: Into<IdError>,
    {
        self.optional.maybe_unit_of_measure(value);
        self
    }

    /// The `ScontoMaggiorazione` blocks, in document order.
    pub fn sconti<G>(mut self, value: G) -> Self
    where
        Vec<Sconto>: TryFrom<G>,
        <Vec<Sconto> as TryFrom<G>>::Error: Into<IdError>,
    {
        self.optional.sconti(value);
        self
    }
}

impl<P> SevenTerLineBuilder<typestate::Unset, P> {
    /// The `Descrizione`, converted from `value`.
    pub fn description<G>(mut self, value: G) -> SevenTerLineBuilder<typestate::Set, P>
    where
        Description: TryFrom<G>,
        <Description as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(description) = convert(&mut self.optional.error, "description", value) {
            self.description = Some(description);
        }
        self.advance()
    }
}

impl<D> SevenTerLineBuilder<D, typestate::Unset> {
    /// The `PrezzoUnitario`, in the currency the invoice carries,
    /// converted from `value`.
    pub fn unit_price<G>(mut self, value: G) -> SevenTerLineBuilder<D, typestate::Set>
    where
        Amount: TryFrom<G>,
        <Amount as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(unit_price) = convert(&mut self.optional.error, "unit_price", value) {
            self.unit_price = Some(unit_price);
        }
        self.advance()
    }
}

impl SevenTerLineBuilder<typestate::Set, typestate::Set> {
    /// Finishes building the line.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the setters.
    ///
    /// # Panics
    ///
    /// Panics if a required field is unset, which the typestate of the
    /// setter chain prevents.
    pub fn build(self) -> Result<SevenTerLine, BuildError> {
        if let Some(error) = self.optional.error {
            return Err(error);
        }
        Ok(SevenTerLine {
            description: required(self.description),
            quantity: self.optional.quantity,
            unit_of_measure: self.optional.unit_of_measure,
            unit_price: required(self.unit_price),
            sconti: self.optional.sconti.unwrap_or_default(),
        })
    }
}

impl SevenTerLine {
    /// The builder of a [`SevenTerLine`], which checks the required
    /// fields at compile time.
    pub const fn builder() -> SevenTerLineBuilder {
        SevenTerLineBuilder {
            description: None,
            optional: LineOptionals::new(),
            unit_price: None,
            _state: PhantomData,
        }
    }

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Invoice;

/// The builder of the domestic invoice, which checks the required fields
/// at compile time and defers the first conversion error of the setters
/// to `build`.
///
/// The `progressivo`, the `date` and the `numero` are required, and the
/// type parameters `P`, `D` and `N` carry the marker of each one, the
/// `Set` marker once its setter was called, so a document missing a
/// required field does not compile and the `build` is available only when
/// every required field is set. The optional fields, the `lines`, the
/// `causali`, the `currency` and the `bollo`, carry plain slots with
/// their setters available in every state.
#[must_use = "the builder does nothing until you call `build()` on it to finish building"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvoiceDomesticBuilder<
    'a,
    'b,
    P = typestate::Unset,
    D = typestate::Unset,
    N = typestate::Unset,
> {
    /// The seller of the invoice.
    seller: &'a Seller,
    /// The buyer of the invoice.
    buyer: &'b Buyer,
    /// The recipient channel of the buyer.
    recipient: Recipient,
    /// The first line of the invoice.
    line: Line,
    /// The `ProgressivoInvio` of the transmission.
    progressivo: Option<ProgressivoInvio>,
    /// The `Data` of the document.
    date: Option<Date>,
    /// The `Numero` of the document.
    numero: Option<Numero>,
    /// The remaining lines, in document order.
    lines: Option<Vec<Line>>,
    /// The `Causale` texts of the document.
    causali: Option<Vec<Causale>>,
    /// The currency of the document.
    currency: Option<InvoiceCurrency>,
    /// Whether to write the virtual stamp duty.
    bollo: Option<bool>,
    /// The first conversion error of the setters, if any.
    error: Option<BuildError>,
    /// The typestate the builder has reached.
    _state: PhantomData<(P, D, N)>,
}

impl<'a, 'b, P, D, N> InvoiceDomesticBuilder<'a, 'b, P, D, N> {
    /// The builder moved into the typestate the setter advanced to.
    fn advance<NP, ND, NN>(self) -> InvoiceDomesticBuilder<'a, 'b, NP, ND, NN> {
        let Self {
            seller,
            buyer,
            recipient,
            line,
            progressivo,
            date,
            numero,
            lines,
            causali,
            currency,
            bollo,
            error,
            ..
        } = self;
        InvoiceDomesticBuilder {
            seller,
            buyer,
            recipient,
            line,
            progressivo,
            date,
            numero,
            lines,
            causali,
            currency,
            bollo,
            error,
            _state: PhantomData,
        }
    }

    /// The remaining lines, in document order.
    pub fn lines<G>(mut self, value: G) -> Self
    where
        Vec<Line>: TryFrom<G>,
        <Vec<Line> as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(lines) = convert(&mut self.error, "lines", value) {
            self.lines = Some(lines);
        }
        self
    }

    /// The `Causale` texts of the document.
    pub fn causali<G>(mut self, value: G) -> Self
    where
        Vec<Causale>: TryFrom<G>,
        <Vec<Causale> as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(causali) = convert(&mut self.error, "causali", value) {
            self.causali = Some(causali);
        }
        self
    }

    /// The currency of the document.
    pub fn currency<G>(mut self, value: G) -> Self
    where
        InvoiceCurrency: TryFrom<G>,
        <InvoiceCurrency as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(currency) = convert(&mut self.error, "currency", value) {
            self.currency = Some(currency);
        }
        self
    }

    /// Enables the virtual stamp duty, which writes the `DatiBollo` block
    /// with `BolloVirtuale SI` and the 2.00 euros `ImportoBollo` when the
    /// computed total exceeds 77.47 euros, the exclusive threshold of the
    /// DPR 642/72, and omits the block at or below it.
    pub fn bollo<G>(mut self, value: G) -> Self
    where
        bool: TryFrom<G>,
        <bool as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(bollo) = convert(&mut self.error, "bollo", value) {
            self.bollo = Some(bollo);
        }
        self
    }
}

impl<'a, 'b, D, N> InvoiceDomesticBuilder<'a, 'b, typestate::Unset, D, N> {
    /// The `ProgressivoInvio` of the transmission, converted from
    /// `value`.
    pub fn progressivo<G>(
        mut self,
        value: G,
    ) -> InvoiceDomesticBuilder<'a, 'b, typestate::Set, D, N>
    where
        ProgressivoInvio: TryFrom<G>,
        <ProgressivoInvio as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(progressivo) = convert(&mut self.error, "progressivo", value) {
            self.progressivo = Some(progressivo);
        }
        self.advance()
    }
}

impl<'a, 'b, P, N> InvoiceDomesticBuilder<'a, 'b, P, typestate::Unset, N> {
    /// The `Data` of the document, converted from `value`.
    pub fn date<G>(mut self, value: G) -> InvoiceDomesticBuilder<'a, 'b, P, typestate::Set, N>
    where
        Date: TryFrom<G>,
        <Date as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(date) = convert(&mut self.error, "date", value) {
            self.date = Some(date);
        }
        self.advance()
    }
}

impl<'a, 'b, P, D> InvoiceDomesticBuilder<'a, 'b, P, D, typestate::Unset> {
    /// The `Numero` of the document, converted from `value`.
    pub fn numero<G>(mut self, value: G) -> InvoiceDomesticBuilder<'a, 'b, P, D, typestate::Set>
    where
        Numero: TryFrom<G>,
        <Numero as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(numero) = convert(&mut self.error, "numero", value) {
            self.numero = Some(numero);
        }
        self.advance()
    }
}

impl InvoiceDomesticBuilder<'_, '_, typestate::Set, typestate::Set, typestate::Set> {
    /// Finishes building the document, which runs the `SDI` checks of
    /// `validate::validate` before returning it.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the setters, with
    /// the `BuildError::Amount` when a line amount does not fit the XSD
    /// amount types, or with every `SDI` violation when the assembled
    /// document fails a check.
    ///
    /// # Panics
    ///
    /// Panics if a required field is unset, which the typestate of the
    /// setter chain prevents.
    pub fn build(self) -> Result<FatturaElettronica, BuildError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        build_invoice(
            self.seller.cedente(),
            self.seller.id_trasmittente(),
            self.buyer.party(),
            &Scenario::Domestic {
                recipient: self.recipient,
            },
            TipoDocumentoType::Td01,
            required(self.progressivo),
            required(self.date),
            required(self.numero),
            self.causali.unwrap_or_default(),
            self.bollo.unwrap_or(false),
            first_lines(self.line, self.lines.unwrap_or_default()),
            self.currency.unwrap_or_default(),
            Vec::new(),
            Vec::new(),
        )
    }
}

/// The builder of the art. 7-ter invoice, which checks the required
/// fields at compile time and defers the first conversion error of the
/// setters to `build`.
///
/// The `progressivo`, the `date` and the `numero` are required, and the
/// type parameters `P`, `D` and `N` carry the marker of each one, the
/// `Set` marker once its setter was called, so a document missing a
/// required field does not compile and the `build` is available only when
/// every required field is set. The optional fields, the `lines`, the
/// `causali` and the `currency`, carry plain slots with their setters
/// available in every state.
#[must_use = "the builder does nothing until you call `build()` on it to finish building"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvoiceCrossBorderServicesBuilder<
    'a,
    'b,
    P = typestate::Unset,
    D = typestate::Unset,
    N = typestate::Unset,
> {
    /// The seller of the invoice.
    seller: &'a Seller,
    /// The EU or non-EU business client of the invoice.
    buyer: &'b Business,
    /// The first line of the invoice.
    line: SevenTerLine,
    /// The `ProgressivoInvio` of the transmission.
    progressivo: Option<ProgressivoInvio>,
    /// The `Data` of the document.
    date: Option<Date>,
    /// The `Numero` of the document.
    numero: Option<Numero>,
    /// The remaining lines, in document order.
    lines: Option<Vec<SevenTerLine>>,
    /// The `Causale` texts of the document.
    causali: Option<Vec<Causale>>,
    /// The currency of the document.
    currency: Option<InvoiceCurrency>,
    /// The first conversion error of the setters, if any.
    error: Option<BuildError>,
    /// The typestate the builder has reached.
    _state: PhantomData<(P, D, N)>,
}

impl<'a, 'b, P, D, N> InvoiceCrossBorderServicesBuilder<'a, 'b, P, D, N> {
    /// The builder moved into the typestate the setter advanced to.
    fn advance<NP, ND, NN>(self) -> InvoiceCrossBorderServicesBuilder<'a, 'b, NP, ND, NN> {
        let Self {
            seller,
            buyer,
            line,
            progressivo,
            date,
            numero,
            lines,
            causali,
            currency,
            error,
            ..
        } = self;
        InvoiceCrossBorderServicesBuilder {
            seller,
            buyer,
            line,
            progressivo,
            date,
            numero,
            lines,
            causali,
            currency,
            error,
            _state: PhantomData,
        }
    }

    /// The remaining lines, in document order.
    pub fn lines<G>(mut self, value: G) -> Self
    where
        Vec<SevenTerLine>: TryFrom<G>,
        <Vec<SevenTerLine> as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(lines) = convert(&mut self.error, "lines", value) {
            self.lines = Some(lines);
        }
        self
    }

    /// The `Causale` texts of the document.
    pub fn causali<G>(mut self, value: G) -> Self
    where
        Vec<Causale>: TryFrom<G>,
        <Vec<Causale> as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(causali) = convert(&mut self.error, "causali", value) {
            self.causali = Some(causali);
        }
        self
    }

    /// The currency of the document.
    pub fn currency<G>(mut self, value: G) -> Self
    where
        InvoiceCurrency: TryFrom<G>,
        <InvoiceCurrency as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(currency) = convert(&mut self.error, "currency", value) {
            self.currency = Some(currency);
        }
        self
    }
}

impl<'a, 'b, D, N> InvoiceCrossBorderServicesBuilder<'a, 'b, typestate::Unset, D, N> {
    /// The `ProgressivoInvio` of the transmission, converted from
    /// `value`.
    pub fn progressivo<G>(
        mut self,
        value: G,
    ) -> InvoiceCrossBorderServicesBuilder<'a, 'b, typestate::Set, D, N>
    where
        ProgressivoInvio: TryFrom<G>,
        <ProgressivoInvio as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(progressivo) = convert(&mut self.error, "progressivo", value) {
            self.progressivo = Some(progressivo);
        }
        self.advance()
    }
}

impl<'a, 'b, P, N> InvoiceCrossBorderServicesBuilder<'a, 'b, P, typestate::Unset, N> {
    /// The `Data` of the document, converted from `value`.
    pub fn date<G>(
        mut self,
        value: G,
    ) -> InvoiceCrossBorderServicesBuilder<'a, 'b, P, typestate::Set, N>
    where
        Date: TryFrom<G>,
        <Date as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(date) = convert(&mut self.error, "date", value) {
            self.date = Some(date);
        }
        self.advance()
    }
}

impl<'a, 'b, P, D> InvoiceCrossBorderServicesBuilder<'a, 'b, P, D, typestate::Unset> {
    /// The `Numero` of the document, converted from `value`.
    pub fn numero<G>(
        mut self,
        value: G,
    ) -> InvoiceCrossBorderServicesBuilder<'a, 'b, P, D, typestate::Set>
    where
        Numero: TryFrom<G>,
        <Numero as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(numero) = convert(&mut self.error, "numero", value) {
            self.numero = Some(numero);
        }
        self.advance()
    }
}

impl InvoiceCrossBorderServicesBuilder<'_, '_, typestate::Set, typestate::Set, typestate::Set> {
    /// Finishes building the document, which runs the `SDI` checks of
    /// `validate::validate` before returning it.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the setters, with
    /// the `BuildError::Amount` when a line amount does not fit the XSD
    /// amount types, or with every `SDI` violation when the assembled
    /// document fails a check.
    ///
    /// # Panics
    ///
    /// Panics if a required field is unset, which the typestate of the
    /// setter chain prevents.
    pub fn build(self) -> Result<FatturaElettronica, BuildError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        build_invoice(
            self.seller.cedente(),
            self.seller.id_trasmittente(),
            self.buyer.party(),
            &Scenario::CrossBorder,
            TipoDocumentoType::Td01,
            required(self.progressivo),
            required(self.date),
            required(self.numero),
            self.causali.unwrap_or_default(),
            true,
            seven_ter_lines(first_seven_ter_lines(
                self.line,
                self.lines.unwrap_or_default(),
            )),
            self.currency.unwrap_or_default(),
            Vec::new(),
            Vec::new(),
        )
    }
}

impl Invoice {
    /// The domestic `TD01` invoice of a resident seller, which checks the
    /// required fields at compile time and defers the first conversion
    /// error of the setters to `build`. The `FPR12` format carries the
    /// seven-character `Recipient::Codice`, the `FPA12` the
    /// six-character code of a public administration.
    pub const fn domestic<'a, 'b>(
        seller: &'a Seller,
        buyer: &'b Buyer,
        recipient: Recipient,
        line: Line,
    ) -> InvoiceDomesticBuilder<'a, 'b> {
        InvoiceDomesticBuilder {
            seller,
            buyer,
            recipient,
            line,
            progressivo: None,
            date: None,
            numero: None,
            lines: None,
            causali: None,
            currency: None,
            bollo: None,
            error: None,
            _state: PhantomData,
        }
    }

    /// The art. 7-ter services invoice to an EU or non-EU business
    /// client, which checks the required fields at compile time and
    /// defers the first conversion error of the setters to `build`.
    pub const fn cross_border_services<'a, 'b>(
        seller: &'a Seller,
        buyer: &'b Business,
        line: SevenTerLine,
    ) -> InvoiceCrossBorderServicesBuilder<'a, 'b> {
        InvoiceCrossBorderServicesBuilder {
            seller,
            buyer,
            line,
            progressivo: None,
            date: None,
            numero: None,
            lines: None,
            causali: None,
            currency: None,
            error: None,
            _state: PhantomData,
        }
    }
}

/// A self-invoice of a buyer integrating the reverse charge of a foreign
/// supplier, the `TD17` of services and the `TD19` of goods.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SelfInvoice;

/// The document kinds of the self-invoice, the `TD17` of the services and
/// the `TD19` of the goods.
#[doc(hidden)]
pub mod kind {
    /// The `TD17` document kind, the services bought from a non-Italian
    /// supplier.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Td17;

    /// The `TD19` document kind, the goods bought from a non-Italian
    /// supplier.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Td19;

    impl super::sealed::Sealed for Td17 {}
    impl super::sealed::Sealed for Td19 {}
}

/// The unnameable supertrait that seals [`SelfInvoiceKind`] to the kinds of
/// [`kind`].
mod sealed {
    /// Implemented by the two document kinds only, since no other crate
    /// can name it.
    pub trait Sealed {}
}

/// The document kind of the self-invoice, carrying the `TipoDocumento`
/// the `build` of the kind's builder writes.
///
/// The kind is the type parameter `K` of the [`SelfInvoiceBuilder`]. The
/// trait is sealed, and the constructors [`SelfInvoice::td17`] and
/// [`SelfInvoice::td19`] are the only way a builder of a kind is produced.
#[doc(hidden)]
pub trait SelfInvoiceKind: sealed::Sealed {
    /// The `TipoDocumento` the kind writes.
    fn tipo_documento() -> TipoDocumentoType;
}

impl SelfInvoiceKind for kind::Td17 {
    fn tipo_documento() -> TipoDocumentoType {
        TipoDocumentoType::Td17
    }
}

impl SelfInvoiceKind for kind::Td19 {
    fn tipo_documento() -> TipoDocumentoType {
        TipoDocumentoType::Td19
    }
}

/// The builder of the `TD17` and the `TD19` self-invoice, which checks
/// the required fields at compile time and defers the first conversion
/// error of the setters to `build`.
///
/// The type parameter `K` carries the document kind, the `Td17` of the
/// services and the `Td19` of the goods, which the constructor
/// [`SelfInvoice::td17`] or [`SelfInvoice::td19`] chose, and which the
/// `build` reads for its `TipoDocumento`. The `progressivo`, the `date`
/// and the `numero` are required, and the type parameters `P`, `D` and
/// `N` carry the marker of each one, the `Set` marker once its setter was
/// called, so a document missing a required field does not compile and
/// the `build` is available only when every required field is set. The
/// optional fields, the `lines`, the `causali`, the `currency` and the
/// `original_invoice`, carry plain slots with their setters available in
/// every state.
#[must_use = "the builder does nothing until you call `build()` on it to finish building"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelfInvoiceBuilder<
    'a,
    'b,
    K,
    P = typestate::Unset,
    D = typestate::Unset,
    N = typestate::Unset,
> {
    /// The non-Italian supplier of the service or the goods.
    supplier: &'a Business,
    /// The Italian buyer with its `partita IVA`.
    buyer: &'b Company,
    /// The recipient channel of the buyer.
    recipient: Recipient,
    /// The first line of the self-invoice, at the buyer's rate.
    line: Line,
    /// The `ProgressivoInvio` of the transmission.
    progressivo: Option<ProgressivoInvio>,
    /// The `Data` of the document.
    date: Option<Date>,
    /// The `Numero` of the document.
    numero: Option<Numero>,
    /// The remaining lines, in document order.
    lines: Option<Vec<Line>>,
    /// The `Causale` texts of the document.
    causali: Option<Vec<Causale>>,
    /// The currency of the document.
    currency: Option<InvoiceCurrency>,
    /// The `DatiFattureCollegate` to the original document, its
    /// `IdDocumento` and `Data`.
    original_invoice: Option<(IdDocumento, Date)>,
    /// The first conversion error of the setters, if any.
    error: Option<BuildError>,
    /// The typestate the builder has reached.
    _state: PhantomData<(K, P, D, N)>,
}

impl<'a, 'b, K, P, D, N> SelfInvoiceBuilder<'a, 'b, K, P, D, N>
where
    K: SelfInvoiceKind,
{
    /// The builder moved into the typestate the setter advanced to.
    fn advance<NP, ND, NN>(self) -> SelfInvoiceBuilder<'a, 'b, K, NP, ND, NN> {
        let Self {
            supplier,
            buyer,
            recipient,
            line,
            progressivo,
            date,
            numero,
            lines,
            causali,
            currency,
            original_invoice,
            error,
            ..
        } = self;
        SelfInvoiceBuilder {
            supplier,
            buyer,
            recipient,
            line,
            progressivo,
            date,
            numero,
            lines,
            causali,
            currency,
            original_invoice,
            error,
            _state: PhantomData,
        }
    }

    /// The remaining lines, in document order.
    pub fn lines<G>(mut self, value: G) -> Self
    where
        Vec<Line>: TryFrom<G>,
        <Vec<Line> as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(lines) = convert(&mut self.error, "lines", value) {
            self.lines = Some(lines);
        }
        self
    }

    /// The `Causale` texts of the document.
    pub fn causali<G>(mut self, value: G) -> Self
    where
        Vec<Causale>: TryFrom<G>,
        <Vec<Causale> as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(causali) = convert(&mut self.error, "causali", value) {
            self.causali = Some(causali);
        }
        self
    }

    /// The currency of the document.
    pub fn currency<G>(mut self, value: G) -> Self
    where
        InvoiceCurrency: TryFrom<G>,
        <InvoiceCurrency as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(currency) = convert(&mut self.error, "currency", value) {
            self.currency = Some(currency);
        }
        self
    }
}

impl<'a, 'b, K, D, N> SelfInvoiceBuilder<'a, 'b, K, typestate::Unset, D, N>
where
    K: SelfInvoiceKind,
{
    /// The `ProgressivoInvio` of the transmission, converted from
    /// `value`.
    pub fn progressivo<G>(mut self, value: G) -> SelfInvoiceBuilder<'a, 'b, K, typestate::Set, D, N>
    where
        ProgressivoInvio: TryFrom<G>,
        <ProgressivoInvio as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(progressivo) = convert(&mut self.error, "progressivo", value) {
            self.progressivo = Some(progressivo);
        }
        self.advance()
    }
}

impl<'a, 'b, K, P, N> SelfInvoiceBuilder<'a, 'b, K, P, typestate::Unset, N>
where
    K: SelfInvoiceKind,
{
    /// The `Data` of the document, converted from `value`.
    pub fn date<G>(mut self, value: G) -> SelfInvoiceBuilder<'a, 'b, K, P, typestate::Set, N>
    where
        Date: TryFrom<G>,
        <Date as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(date) = convert(&mut self.error, "date", value) {
            self.date = Some(date);
        }
        self.advance()
    }
}

impl<'a, 'b, K, P, D> SelfInvoiceBuilder<'a, 'b, K, P, D, typestate::Unset>
where
    K: SelfInvoiceKind,
{
    /// The `Numero` of the document, converted from `value`.
    pub fn numero<G>(mut self, value: G) -> SelfInvoiceBuilder<'a, 'b, K, P, D, typestate::Set>
    where
        Numero: TryFrom<G>,
        <Numero as TryFrom<G>>::Error: Into<IdError>,
    {
        if let Some(numero) = convert(&mut self.error, "numero", value) {
            self.numero = Some(numero);
        }
        self.advance()
    }
}

impl<K, P, D, N> SelfInvoiceBuilder<'_, '_, K, P, D, N> {
    /// The `DatiFattureCollegate` to the original document, its
    /// `IdDocumento` and `Data`, converted.
    pub fn original_invoice<Id, Dt>(mut self, numero: Id, data: Dt) -> Self
    where
        IdDocumento: TryFrom<Id>,
        <IdDocumento as TryFrom<Id>>::Error: Into<IdError>,
        Date: TryFrom<Dt>,
        <Date as TryFrom<Dt>>::Error: Into<IdError>,
    {
        if let Some(numero) = convert(&mut self.error, "original_invoice", numero)
            && let Some(data) = convert(&mut self.error, "original_invoice", data)
        {
            self.original_invoice = Some((numero, data));
        }
        self
    }
}

impl<K> SelfInvoiceBuilder<'_, '_, K, typestate::Set, typestate::Set, typestate::Set>
where
    K: SelfInvoiceKind,
{
    /// Finishes building the document, which runs the `SDI` checks of
    /// `validate::validate` before returning it.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the setters, with
    /// the `BuildError::Amount` when a line amount does not fit the XSD
    /// amount types, or with every `SDI` violation when the assembled
    /// document fails a check.
    ///
    /// # Panics
    ///
    /// Panics if a required field is unset, which the typestate of the
    /// setter chain prevents.
    pub fn build(self) -> Result<FatturaElettronica, BuildError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        let date = required(self.date);
        build_invoice(
            self.supplier.cedente(RegimeFiscale::Rf01),
            self.buyer.id_trasmittente(),
            self.buyer.party(),
            &Scenario::Domestic {
                recipient: self.recipient,
            },
            K::tipo_documento(),
            required(self.progressivo),
            date,
            required(self.numero),
            self.causali.unwrap_or_default(),
            false,
            first_lines(self.line, self.lines.unwrap_or_default()),
            self.currency.unwrap_or_default(),
            vec![pagamento(date)],
            self.original_invoice
                .map(|(numero, data)| collegate(&numero, data))
                .into_iter()
                .collect(),
        )
    }
}

impl SelfInvoice {
    /// The `TD17` self-invoice of a service bought from a non-Italian
    /// supplier, which checks the required fields at compile time and
    /// defers the first conversion error of the setters to `build`.
    pub const fn td17<'a, 'b>(
        supplier: &'a Business,
        buyer: &'b Company,
        recipient: Recipient,
        line: Line,
    ) -> SelfInvoiceBuilder<'a, 'b, kind::Td17> {
        Self::builder(supplier, buyer, recipient, line)
    }

    /// The `TD19` self-invoice of goods bought from a non-Italian
    /// supplier, which checks the required fields at compile time and
    /// defers the first conversion error of the setters to `build`.
    pub const fn td19<'a, 'b>(
        supplier: &'a Business,
        buyer: &'b Company,
        recipient: Recipient,
        line: Line,
    ) -> SelfInvoiceBuilder<'a, 'b, kind::Td19> {
        Self::builder(supplier, buyer, recipient, line)
    }

    /// The builder of the self-invoice, the document kind the constructor
    /// names in the type parameter `K`.
    const fn builder<'a, 'b, K: SelfInvoiceKind>(
        supplier: &'a Business,
        buyer: &'b Company,
        recipient: Recipient,
        line: Line,
    ) -> SelfInvoiceBuilder<'a, 'b, K> {
        SelfInvoiceBuilder {
            supplier,
            buyer,
            recipient,
            line,
            progressivo: None,
            date: None,
            numero: None,
            lines: None,
            causali: None,
            currency: None,
            original_invoice: None,
            error: None,
            _state: PhantomData,
        }
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
    let bollo = if bollo && importo_totale > BOLLO_THRESHOLD {
        Some(DatiBolloType {
            content: DatiBolloTypeContent {
                bollo_virtuale: BolloVirtualeType::Si,
                importo_bollo: Some(Amount(BOLLO_IMPORT)),
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
    use crate::ids::{CodiceFiscale, IdError, PartitaIva, Provincia, UnitaMisura};
    use crate::party::{Address, TaxIdentity};
    use iso_country::Country;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
            .description("MERCE")
            .quantity(Amount(Decimal::new(2, 2)))
            .unit_price(Amount(Decimal::new(10000, 2)))
            .vat(VatTreatment::NotTaxed {
                natura: Natura::N3_2,
                riferimento_normativo: reference.parse().expect("a valid reference"),
            })
            .build()
            .expect("the line builds")
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
            .description("SERVIZIO DI CONSULENZA")
            .quantity(Amount(Decimal::new(10000, 2)))
            .unit_price(Amount(Decimal::new(15000, 2)))
            .build()
            .expect("the line builds")
    }

    fn taxed_line(value: i64) -> Line {
        Line::builder()
            .description("SERVIZIO")
            .quantity(Amount(Decimal::new(100, 2)))
            .unit_price(Amount(Decimal::new(value, 2)))
            .vat(VatTreatment::Taxed(
                "22.00".parse::<Aliquota>().expect("a valid rate"),
            ))
            .build()
            .expect("the line builds")
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
        .date("2026-11-05")
        .numero("001")
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
            .numero("00011")
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
        .date("2026-11-05")
        .numero("001")
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
            .description("BENE")
            .quantity(Amount(Decimal::new(100, 2)))
            .unit_of_measure(UnitaMisura::try_new("KG").expect("a valid unit of measure"))
            .unit_price(Amount(Decimal::new(1000, 2)))
            .vat(VatTreatment::Taxed(
                "22.00".parse::<Aliquota>().expect("a valid rate"),
            ))
            .build()
            .expect("the line builds");
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
            .description("SERVIZIO")
            .quantity(Amount(Decimal::new(100, 2)))
            .unit_price(Amount(Decimal::new(100_000_001, 8)))
            .vat(VatTreatment::Taxed(
                "22.00".parse::<Aliquota>().expect("a valid rate"),
            ))
            .build()
            .expect("the line builds");
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
            .description("SERVIZIO")
            .quantity(Amount(Decimal::new(100, 2)))
            .unit_price(Amount(Decimal::new(100_000_000_001, 9)))
            .vat(VatTreatment::Taxed(
                "22.00".parse::<Aliquota>().expect("a valid rate"),
            ))
            .build()
            .expect("the line builds");
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
            .description("SERVIZIO")
            .quantity(Amount(Decimal::new(2, 0)))
            .unit_price(Amount(Decimal::new(55, 1)))
            .vat(VatTreatment::Taxed(
                "22.00".parse::<Aliquota>().expect("a valid rate"),
            ))
            .build()
            .expect("the line builds");
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

    /// A source whose `TryFrom` conversions are counted, so a test can
    /// prove that a setter skips the conversion after the first error.
    #[derive(Clone)]
    struct Counting {
        /// The counter of the conversions.
        counter: &'static AtomicUsize,
        /// The text the source converts.
        value: &'static str,
    }

    impl TryFrom<Counting> for Description {
        type Error = IdError;

        fn try_from(source: Counting) -> Result<Self, IdError> {
            source.counter.fetch_add(1, Ordering::Relaxed);
            Self::try_new(source.value)
        }
    }

    impl TryFrom<Counting> for Amount {
        type Error = IdError;

        fn try_from(source: Counting) -> Result<Self, IdError> {
            source.counter.fetch_add(1, Ordering::Relaxed);
            Self::try_from(source.value)
        }
    }

    impl TryFrom<Counting> for ProgressivoInvio {
        type Error = IdError;

        fn try_from(source: Counting) -> Result<Self, IdError> {
            source.counter.fetch_add(1, Ordering::Relaxed);
            Self::try_new(source.value)
        }
    }

    impl TryFrom<Counting> for Date {
        type Error = IdError;

        fn try_from(source: Counting) -> Result<Self, IdError> {
            source.counter.fetch_add(1, Ordering::Relaxed);
            Self::try_from(source.value)
        }
    }

    /// A setter whose conversion fails is reported by `build` with the
    /// name of the setter, although the remaining required fields are
    /// set after it.
    #[test]
    fn a_failing_setter_is_reported_at_build() {
        let line = Line::builder()
            .description("SERVIZIO")
            .unit_price("not a decimal")
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build();
        assert!(
            matches!(
                line,
                Err(BuildError::Id {
                    field: "unit_price",
                    error: IdError::Amount(..)
                })
            ),
            "{line:?}"
        );
    }

    /// When two setters fail, `build` reports the first error.
    #[test]
    fn the_first_conversion_error_wins() {
        let line = Line::builder()
            .description("")
            .unit_price("not a decimal")
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build();
        assert!(
            matches!(
                line,
                Err(BuildError::Id {
                    field: "description",
                    ..
                })
            ),
            "{line:?}"
        );
    }

    /// A typed value and the equivalent string build the same line.
    #[test]
    fn typed_and_string_values_build_the_same_line() {
        let typed = Line::builder()
            .description(Description::try_new("SERVIZIO").expect("a valid description"))
            .quantity(Amount(Decimal::new(100, 2)))
            .unit_price(Amount(Decimal::new(10_000, 2)))
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build()
            .expect("the line builds");
        let strings = Line::builder()
            .description("SERVIZIO")
            .quantity("1.00")
            .unit_price("100.00")
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build()
            .expect("the line builds");
        assert_eq!(typed, strings);
    }

    /// A typed value and the equivalent string build the same document.
    #[test]
    fn typed_and_string_values_build_the_same_invoice() {
        let typed = Invoice::domestic(&seller(), &buyer_rome(), codice(), taxed_line(10_000))
            .progressivo(progressivo())
            .date(date())
            .numero(numero())
            .build()
            .expect("the invoice builds");
        let strings = Invoice::domestic(&seller(), &buyer_rome(), codice(), taxed_line(10_000))
            .progressivo("00001")
            .date("2026-11-04")
            .numero("00012")
            .build()
            .expect("the invoice builds");
        assert_eq!(typed, strings);
    }

    /// Once a setter fails, the later setters skip their conversion and
    /// only advance the typestate.
    #[test]
    fn a_later_setter_skips_the_conversion_after_the_first_error() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let line = Line::builder()
            .description(Counting {
                counter: &COUNTER,
                value: "",
            })
            .unit_price(Counting {
                counter: &COUNTER,
                value: "1.00",
            })
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build();
        assert_eq!(COUNTER.load(Ordering::Relaxed), 1);
        assert!(
            matches!(
                line,
                Err(BuildError::Id {
                    field: "description",
                    ..
                })
            ),
            "{line:?}"
        );
    }

    /// Once a setter of the invoice fails, the later setters skip their
    /// conversion and only advance the typestate.
    #[test]
    fn the_invoice_skips_the_conversion_after_the_first_error() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let doc = Invoice::domestic(&seller(), &buyer_rome(), codice(), taxed_line(10_000))
            .progressivo(Counting {
                counter: &COUNTER,
                value: "00001X1",
            })
            .date(Counting {
                counter: &COUNTER,
                value: "2026-11-04",
            })
            .numero("00012")
            .build();
        assert_eq!(COUNTER.load(Ordering::Relaxed), 1);
        assert!(
            matches!(
                doc,
                Err(BuildError::Id {
                    field: "progressivo",
                    ..
                })
            ),
            "{doc:?}"
        );
    }

    /// `build` reports the conversion error before the line amount checks
    /// and the `SDI` checks run.
    #[test]
    fn build_reports_the_conversion_error_before_the_other_checks() {
        let line = Line::builder()
            .description("SERVIZIO")
            .unit_price("1234567890.00")
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build()
            .expect("the line builds");
        let doc = Invoice::domestic(&seller(), &buyer_rome(), codice(), line)
            .progressivo("00001X1")
            .date("2026-11-04")
            .numero("00012")
            .build();
        assert!(
            matches!(
                doc,
                Err(BuildError::Id {
                    field: "progressivo",
                    ..
                })
            ),
            "the id error precedes the amount and the SDI checks: {doc:?}"
        );
    }

    /// The generic constructors of the typed layer report the failing
    /// value with its own `IdError` variant.
    #[test]
    fn the_generic_constructors_report_the_failing_value() {
        assert!(matches!(
            Seller::try_denomination("ALPHA SRL", "00012345675", rome(), RegimeFiscale::Rf01),
            Err(IdError::PartitaIva(..))
        ));
        assert!(matches!(
            Business::try_eu(crate::ids::EuCountry::De, "MUSTER GMBH", "xx", rome()),
            Err(IdError::ForeignTaxId(..))
        ));
        assert!(matches!(
            Address::try_italian("VIA ROMA 1", "0010", "ROMA", Provincia::Rm),
            Err(IdError::Cap(..))
        ));
        assert!(matches!(
            Address::try_foreign("A".repeat(55), Some("12345"), "NEW YORK", Country::US),
            Err(IdError::Address(..))
        ));
        assert!(TaxIdentity::try_italian::<PartitaIva, CodiceFiscale>(None, None).is_err());
        assert!(matches!(
            Recipient::try_codice("ABC"),
            Err(IdError::CodiceDestinatario(..))
        ));
        assert!(matches!(
            InvoiceCurrency::try_foreign(Currency::USD, "0", "2026-11-04"),
            Err(IdError::ExchangeRate(..))
        ));
        assert!(matches!(
            VatTreatment::taxed("0.00"),
            Err(IdError::Aliquota(..))
        ));
        assert!(Recipient::try_pec("not an e-mail").is_err());
    }

    /// The `maybe_*` setters of the line builders store the present
    /// value, omit the absent one and report the failing conversion at
    /// the line build.
    #[expect(
        clippy::too_many_lines,
        reason = "one block per builder per value kind: present, absent, failing"
    )]
    #[test]
    fn the_maybe_setters_carry_the_optional_values() {
        let line = Line::builder()
            .description("SERVIZIO")
            .maybe_quantity(Some("2.00"))
            .maybe_unit_of_measure(Some("KG"))
            .unit_price(Amount(Decimal::new(10000, 2)))
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build()
            .expect("the line builds");
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
            line.quantita.as_ref().expect("the quantity is present").0,
            Decimal::new(200, 2)
        );
        assert_eq!(line.unita_misura.as_deref(), Some("KG"));

        let absent = Line::builder()
            .description("SERVIZIO")
            .maybe_quantity(None::<&str>)
            .maybe_unit_of_measure(None::<&str>)
            .unit_price(Amount(Decimal::new(10000, 2)))
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build()
            .expect("the line builds");
        let doc = Invoice::domestic(&seller(), &buyer_rome(), codice(), absent)
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
        assert_eq!(line.quantita, None);
        assert_eq!(line.unita_misura, None);

        let failing = Line::builder()
            .description("SERVIZIO")
            .maybe_quantity(Some("not a decimal"))
            .unit_price(Amount(Decimal::new(10000, 2)))
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build();
        assert!(
            matches!(
                failing,
                Err(BuildError::Id {
                    field: "quantity",
                    error: IdError::Amount(..)
                })
            ),
            "{failing:?}"
        );
        let failing = Line::builder()
            .description("SERVIZIO")
            .maybe_unit_of_measure(Some("TOO LONG MEASURE"))
            .unit_price(Amount(Decimal::new(10000, 2)))
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build();
        assert!(
            matches!(
                failing,
                Err(BuildError::Id {
                    field: "unit_of_measure",
                    error: IdError::UnitaMisura(..)
                })
            ),
            "{failing:?}"
        );

        let seven_ter = SevenTerLine::builder()
            .description("SERVIZIO")
            .maybe_quantity(Some("3.00"))
            .maybe_unit_of_measure(Some("PEZZO"))
            .unit_price(Amount(Decimal::new(15000, 2)))
            .build()
            .expect("the line builds");
        let doc = Invoice::cross_border_services(&seller(), &buyer_us(), seven_ter)
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
            line.quantita.as_ref().expect("the quantity is present").0,
            Decimal::new(300, 2)
        );
        assert_eq!(line.unita_misura.as_deref(), Some("PEZZO"));

        let seven_ter_absent = SevenTerLine::builder()
            .description("SERVIZIO")
            .maybe_quantity(None::<&str>)
            .maybe_unit_of_measure(None::<&str>)
            .unit_price(Amount(Decimal::new(15000, 2)))
            .build()
            .expect("the line builds");
        let doc = Invoice::cross_border_services(&seller(), &buyer_us(), seven_ter_absent)
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
        assert_eq!(line.quantita, None);
        assert_eq!(line.unita_misura, None);

        let failing = SevenTerLine::builder()
            .description("SERVIZIO")
            .maybe_quantity(Some("not a decimal"))
            .unit_price(Amount(Decimal::new(15000, 2)))
            .build();
        assert!(
            matches!(
                failing,
                Err(BuildError::Id {
                    field: "quantity",
                    error: IdError::Amount(..)
                })
            ),
            "{failing:?}"
        );
        let failing = SevenTerLine::builder()
            .description("SERVIZIO")
            .maybe_unit_of_measure(Some("TOO LONG MEASURE"))
            .unit_price(Amount(Decimal::new(15000, 2)))
            .build();
        assert!(
            matches!(
                failing,
                Err(BuildError::Id {
                    field: "unit_of_measure",
                    error: IdError::UnitaMisura(..)
                })
            ),
            "{failing:?}"
        );
    }

    /// A failing `progressivo` is reported by the `build` of every
    /// document builder, and the later setters skip their conversion.
    #[test]
    fn the_document_builders_report_the_first_setter_error() {
        let domestic = Invoice::domestic(&seller(), &buyer_rome(), codice(), taxed_line(10000))
            .progressivo("00001X1")
            .date("2026-11-04")
            .numero("00012")
            .lines(vec![taxed_line(5000)])
            .causali(vec!["SERVIZI".parse().expect("a valid causale")])
            .currency(InvoiceCurrency::Eur)
            .bollo(true)
            .build();
        assert!(
            matches!(
                domestic,
                Err(BuildError::Id {
                    field: "progressivo",
                    ..
                })
            ),
            "{domestic:?}"
        );
        let cross_border = Invoice::cross_border_services(&seller(), &buyer_us(), seven_ter_line())
            .progressivo("00001X1")
            .date("2026-11-04")
            .numero("00012")
            .lines(vec![seven_ter_line()])
            .causali(vec!["SERVIZI".parse().expect("a valid causale")])
            .currency(InvoiceCurrency::Eur)
            .build();
        assert!(
            matches!(
                cross_border,
                Err(BuildError::Id {
                    field: "progressivo",
                    ..
                })
            ),
            "{cross_border:?}"
        );
        let td17 = SelfInvoice::td17(
            &supplier_de(),
            &buyer_company(),
            codice(),
            taxed_line(10000),
        )
        .progressivo("00001X1")
        .date("2026-11-04")
        .numero("00012")
        .lines(vec![taxed_line(5000)])
        .causali(vec!["SERVIZI".parse().expect("a valid causale")])
        .currency(InvoiceCurrency::Eur)
        .original_invoice("ORIG-001", "2026-11-01")
        .build();
        assert!(
            matches!(
                td17,
                Err(BuildError::Id {
                    field: "progressivo",
                    ..
                })
            ),
            "{td17:?}"
        );
        let td19 = SelfInvoice::td19(
            &supplier_de(),
            &buyer_company(),
            codice(),
            taxed_line(10000),
        )
        .progressivo("00001X1")
        .date("2026-11-04")
        .numero("00012")
        .lines(vec![taxed_line(5000)])
        .causali(vec!["SERVIZI".parse().expect("a valid causale")])
        .currency(InvoiceCurrency::Eur)
        .original_invoice("ORIG-001", "2026-11-01")
        .build();
        assert!(
            matches!(
                td19,
                Err(BuildError::Id {
                    field: "progressivo",
                    ..
                })
            ),
            "{td19:?}"
        );
    }

    /// The `TD17` and `TD19` builders carry the remaining lines, the
    /// `Causale` texts and the `DatiFattureCollegate` of the original
    /// document, and report a failing `original_invoice` conversion.
    #[expect(
        clippy::too_many_lines,
        reason = "one block per builder per field: the lines, the causales, the collegate"
    )]
    #[test]
    fn the_self_invoice_builders_carry_the_optional_fields() {
        let doc = SelfInvoice::td17(
            &supplier_de(),
            &buyer_company(),
            codice(),
            taxed_line(10000),
        )
        .progressivo(progressivo())
        .date(date())
        .numero(numero())
        .lines(vec![taxed_line(5000)])
        .causali(vec!["AUTOFATTURA".parse().expect("a valid causale")])
        .original_invoice("ORIG-001", "2026-10-01")
        .build()
        .expect("the self-invoice builds");
        assert_eq!(validate::validate(&doc), Vec::new());
        let body = &doc.content.fattura_elettronica_body[0].content;
        assert_eq!(body.dati_beni_servizi.content.dettaglio_linee.len(), 2);
        assert_eq!(
            body.dati_generali
                .content
                .dati_generali_documento
                .content
                .causale,
            vec!["AUTOFATTURA".to_string()]
        );
        let collegate = &body.dati_generali.content.dati_fatture_collegate;
        assert_eq!(collegate.len(), 1);
        assert_eq!(collegate[0].content.id_documento, "ORIG-001");
        assert_eq!(
            collegate[0].content.data,
            Some("2026-10-01".parse::<Date>().expect("a valid date"))
        );

        let doc = SelfInvoice::td19(
            &supplier_de(),
            &buyer_company(),
            codice(),
            taxed_line(10000),
        )
        .progressivo(progressivo())
        .date(date())
        .numero(numero())
        .lines(vec![taxed_line(5000)])
        .causali(vec!["AUTOFATTURA".parse().expect("a valid causale")])
        .original_invoice("ORIG-001", "2026-10-01")
        .build()
        .expect("the self-invoice builds");
        assert_eq!(validate::validate(&doc), Vec::new());
        let body = &doc.content.fattura_elettronica_body[0].content;
        assert_eq!(body.dati_beni_servizi.content.dettaglio_linee.len(), 2);
        let collegate = &body.dati_generali.content.dati_fatture_collegate;
        assert_eq!(collegate.len(), 1);
        assert_eq!(collegate[0].content.id_documento, "ORIG-001");

        let failing = SelfInvoice::td17(
            &supplier_de(),
            &buyer_company(),
            codice(),
            taxed_line(10000),
        )
        .progressivo(progressivo())
        .date(date())
        .numero(numero())
        .original_invoice("ORIG-001", "not a date")
        .build();
        assert!(
            matches!(
                failing,
                Err(BuildError::Id {
                    field: "original_invoice",
                    error: IdError::Date(..)
                })
            ),
            "{failing:?}"
        );
        let failing = SelfInvoice::td17(
            &supplier_de(),
            &buyer_company(),
            codice(),
            taxed_line(10000),
        )
        .progressivo(progressivo())
        .date(date())
        .numero(numero())
        .original_invoice("012345678901234567890", "2026-10-01")
        .build();
        assert!(
            matches!(
                failing,
                Err(BuildError::Id {
                    field: "original_invoice",
                    error: IdError::IdDocumento(..)
                })
            ),
            "{failing:?}"
        );
        let failing = SelfInvoice::td19(
            &supplier_de(),
            &buyer_company(),
            codice(),
            taxed_line(10000),
        )
        .progressivo(progressivo())
        .date(date())
        .numero(numero())
        .original_invoice("ORIG-001", "not a date")
        .build();
        assert!(
            matches!(
                failing,
                Err(BuildError::Id {
                    field: "original_invoice",
                    error: IdError::Date(..)
                })
            ),
            "{failing:?}"
        );
    }

    /// The `build` of a self-invoice reports the 00418 of an original
    /// invoice dated after the document; the same date passes.
    #[test]
    fn the_self_invoice_reports_a_later_original_invoice_date() {
        let failing = SelfInvoice::td17(
            &supplier_de(),
            &buyer_company(),
            codice(),
            taxed_line(10000),
        )
        .progressivo(progressivo())
        .date("2026-11-04")
        .numero("00015")
        .original_invoice("ORIG-001", "2026-11-10")
        .build();
        assert!(
            matches!(
                &failing,
                Err(BuildError::Violations(violations))
                    if violations.iter().any(|violation| violation.code == "00418")
            ),
            "{failing:?}"
        );

        let doc = SelfInvoice::td17(
            &supplier_de(),
            &buyer_company(),
            codice(),
            taxed_line(10000),
        )
        .progressivo(progressivo())
        .date("2026-11-04")
        .numero("00015")
        .original_invoice("ORIG-001", "2026-11-04")
        .build()
        .expect("the same-date original invoice passes");
        assert_eq!(validate::validate(&doc), Vec::new());
    }

    /// A line built without its `Quantita` writes the element absent,
    /// the `PrezzoTotale` computed on the implicit quantity of one, and
    /// the domestic builder carries the `Causale` texts.
    #[test]
    fn a_line_without_a_quantity_omits_the_element() {
        let line = Line::builder()
            .description("SERVIZIO")
            .unit_price(Amount(Decimal::new(10000, 2)))
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build()
            .expect("the line builds");
        let doc = Invoice::domestic(&seller(), &buyer_rome(), codice(), line)
            .progressivo(progressivo())
            .date(date())
            .numero(numero())
            .causali(vec!["SERVIZI".parse().expect("a valid causale")])
            .build()
            .expect("the invoice builds");
        assert_eq!(validate::validate(&doc), Vec::new());
        let body = &doc.content.fattura_elettronica_body[0].content;
        let line = &body.dati_beni_servizi.content.dettaglio_linee[0].content;
        assert_eq!(line.quantita, None);
        assert_eq!(line.prezzo_totale.0, Decimal::new(10000, 2));
        assert_eq!(
            body.dati_generali
                .content
                .dati_generali_documento
                .content
                .causale,
            vec!["SERVIZI".to_string()]
        );
    }

    /// The `InvoiceCurrency` conversion reports a failing `rate_date` as
    /// `IdError::Date`.
    #[test]
    fn the_foreign_currency_reports_a_bad_rate_date() {
        assert!(matches!(
            InvoiceCurrency::try_foreign(Currency::USD, "1.08", "not a date"),
            Err(IdError::Date(..))
        ));
    }

    /// The `Sconto` markup percentage checks the value the same way as
    /// the discount percentage.
    #[test]
    fn the_markup_percent_checks_the_value() {
        assert!(Sconto::markup_percent("10.00").is_ok());
        assert!(Sconto::markup_percent(Decimal::ONE_HUNDRED).is_ok());
        assert!(Sconto::markup_percent("0.00").is_err());
        assert!(Sconto::markup_percent("100.01").is_err());
        assert!(Sconto::markup_percent("12.345").is_err());
        assert!(Sconto::markup_percent("not a decimal").is_err());
    }

    /// A `SevenTerLine` built with the plain `unit_of_measure` setter
    /// carries the `UnitaMisura` into the document.
    #[test]
    fn a_seven_ter_line_carries_the_plain_unit_of_measure() {
        let line = SevenTerLine::builder()
            .description("SERVIZIO")
            .quantity("2.00")
            .unit_of_measure("PEZZO")
            .unit_price(Amount(Decimal::new(15000, 2)))
            .build()
            .expect("the line builds");
        let doc = Invoice::cross_border_services(&seller(), &buyer_us(), line)
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
        assert_eq!(line.unita_misura.as_deref(), Some("PEZZO"));
        assert_eq!(
            line.quantita.as_ref().expect("the quantity is present").0,
            Decimal::new(200, 2)
        );
    }

    /// A failing plain setter is reported by the line `build`, and the
    /// later setters skip their conversion.
    #[test]
    fn the_plain_setters_report_the_first_failing_conversion() {
        let failing = Line::builder()
            .description("SERVIZIO")
            .quantity("not a decimal")
            .unit_of_measure("KG")
            .sconti(Vec::<Sconto>::new())
            .unit_price(Amount(Decimal::new(10000, 2)))
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build();
        assert!(
            matches!(
                failing,
                Err(BuildError::Id {
                    field: "quantity",
                    error: IdError::Amount(..)
                })
            ),
            "{failing:?}"
        );
        let failing = SevenTerLine::builder()
            .description("")
            .quantity("2.00")
            .unit_of_measure("KG")
            .sconti(Vec::<Sconto>::new())
            .unit_price(Amount(Decimal::new(15000, 2)))
            .build();
        assert!(
            matches!(
                failing,
                Err(BuildError::Id {
                    field: "description",
                    error: IdError::Description(..)
                })
            ),
            "{failing:?}"
        );
    }

    /// A source carrying its own `TryFrom` implementation flows through
    /// the line and document builders.
    #[test]
    fn the_custom_try_from_sources_flow_into_the_setters() {
        static UNIT_PRICE: AtomicUsize = AtomicUsize::new(0);
        static DATE: AtomicUsize = AtomicUsize::new(0);
        let line = Line::builder()
            .description("SERVIZIO")
            .quantity("1.00")
            .unit_price(Counting {
                counter: &UNIT_PRICE,
                value: "100.00",
            })
            .vat(VatTreatment::taxed("22.00").expect("a valid rate"))
            .build()
            .expect("the line builds");
        let doc = Invoice::domestic(&seller(), &buyer_rome(), codice(), line)
            .progressivo("00001")
            .date(Counting {
                counter: &DATE,
                value: "2026-11-04",
            })
            .numero("00012")
            .build()
            .expect("the invoice builds");
        assert_eq!(UNIT_PRICE.load(Ordering::Relaxed), 1);
        assert_eq!(DATE.load(Ordering::Relaxed), 1);
        assert_eq!(validate::validate(&doc), Vec::new());
    }
}
