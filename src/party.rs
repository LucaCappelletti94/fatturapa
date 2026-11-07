//! The typed parties of the `FatturaPA` header, their roles, their tax
//! identities and their seats.
//!
//! A party fills the `CedentePrestatore` or the `CessionarioCommittente`
//! with the `ISO 3166-1` code in `IdPaese` and `Nazione`, the validated
//! tax identifier in `IdCodice`, and the seat per the form of the address.
//! A foreign seat writes the `CAP` `00000`, omits the `Provincia` and puts
//! the foreign postal code in the `Indirizzo`, as the Agenzia FAQ
//! prescribes.
//!
//! A party is constructed through one of its roles. The `Seller` of an
//! invoice is an Italian party with its `RegimeFiscale`, the `Buyer` of a
//! domestic invoice is an Italian party, the `Business` of a
//! cross-border invoice or of a self-invoice is a non-Italian business,
//! and the `Company` of a self-invoice is an Italian party with its
//! `partita IVA`.

use crate::generated::fpr12::{
    AnagraficaContent60Type, AnagraficaContent60TypeContent, AnagraficaContent61Type,
    AnagraficaContent61TypeContent, AnagraficaContent62Type, AnagraficaContent62TypeContent,
    AnagraficaType, AnagraficaTypeContent, CedentePrestatoreType, CedentePrestatoreTypeContent,
    CessionarioCommittenteType, CessionarioCommittenteTypeContent, DatiAnagraficiCedenteType,
    DatiAnagraficiCedenteTypeContent, DatiAnagraficiCessionarioType,
    DatiAnagraficiCessionarioTypeContent, IdFiscaleType, IdFiscaleTypeContent, IndirizzoType,
    IndirizzoTypeContent,
};
use crate::ids::{
    Cap, CodiceFiscale, Comune, Denomination, EuCountry, ForeignTaxId, IdError, Indirizzo,
    PartitaIva, PersonName, PostalCode, Provincia, RegimeFiscale,
};
use iso_country::Country;

/// The conversion of an optional constructor argument, shared by the
/// `try_*` constructors.
///
/// A present value is converted into `T` and its error mapped to
/// `IdError`, an absent value stays absent.
fn convert_opt<T, G>(value: Option<G>) -> Result<Option<T>, IdError>
where
    T: TryFrom<G>,
    <T as TryFrom<G>>::Error: Into<IdError>,
{
    match value {
        Some(value) => Ok(Some(T::try_from(value).map_err(Into::into)?)),
        None => Ok(None),
    }
}

/// The tax identity of a party, one of the three forms the header admits.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum TaxIdentity {
    /// The identity of an Italian party, `IdPaese` `IT`. At least one of
    /// the two identifiers is present.
    Italian {
        /// The `IdCodice` as `partita IVA`, when present.
        partita_iva: Option<PartitaIva>,
        /// The `CodiceFiscale` element, when present.
        codice_fiscale: Option<CodiceFiscale>,
    },
    /// The identity of an EU business, `IdPaese` the member state and
    /// `IdCodice` the VAT number.
    EuVat {
        /// The member state.
        country: EuCountry,
        /// The VAT number.
        vat: ForeignTaxId,
    },
    /// The identity of a non-EU party, `IdPaese` the country and
    /// `IdCodice` the fiscal identifier of residence.
    Foreign {
        /// The country of residence.
        country: Country,
        /// The fiscal identifier of residence.
        id: ForeignTaxId,
    },
}

impl TaxIdentity {
    /// Builds the Italian identity when at least one identifier is
    /// present, each converted from its value.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the identifiers,
    /// or when both identifiers are absent.
    pub fn try_italian<P, C>(
        partita_iva: Option<P>,
        codice_fiscale: Option<C>,
    ) -> Result<Self, IdError>
    where
        PartitaIva: TryFrom<P>,
        <PartitaIva as TryFrom<P>>::Error: Into<IdError>,
        CodiceFiscale: TryFrom<C>,
        <CodiceFiscale as TryFrom<C>>::Error: Into<IdError>,
    {
        let partita_iva = convert_opt(partita_iva)?;
        let codice_fiscale = convert_opt(codice_fiscale)?;
        if partita_iva.is_some() || codice_fiscale.is_some() {
            Ok(Self::Italian {
                partita_iva,
                codice_fiscale,
            })
        } else {
            Err(IdError::CodiceFiscale(String::new()))
        }
    }

    /// Builds the EU business identity.
    #[must_use]
    pub const fn eu_vat(country: EuCountry, vat: ForeignTaxId) -> Self {
        Self::EuVat { country, vat }
    }

    /// Builds the EU business identity, each value converted.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments.
    pub fn try_eu_vat<C, V>(country: C, vat: V) -> Result<Self, IdError>
    where
        EuCountry: TryFrom<C>,
        <EuCountry as TryFrom<C>>::Error: Into<IdError>,
        ForeignTaxId: TryFrom<V>,
        <ForeignTaxId as TryFrom<V>>::Error: Into<IdError>,
    {
        Ok(Self::eu_vat(
            EuCountry::try_from(country).map_err(Into::into)?,
            ForeignTaxId::try_from(vat).map_err(Into::into)?,
        ))
    }

    /// Builds the non-EU identity.
    #[must_use]
    pub const fn foreign(country: Country, id: ForeignTaxId) -> Self {
        Self::Foreign { country, id }
    }

    /// Builds the non-EU identity, the values converted. The `country`
    /// is the `ISO 3166-1` code the caller resolves to the `Country`
    /// enum.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments.
    pub fn try_foreign<I>(country: Country, id: I) -> Result<Self, IdError>
    where
        ForeignTaxId: TryFrom<I>,
        <ForeignTaxId as TryFrom<I>>::Error: Into<IdError>,
    {
        Ok(Self::foreign(
            country,
            ForeignTaxId::try_from(id).map_err(Into::into)?,
        ))
    }

    /// The `IdFiscaleIVA` pair, `IdPaese` and `IdCodice`, when the
    /// identity writes the element.
    #[must_use]
    pub fn id_fiscale_iva(&self) -> Option<(String, String)> {
        match self {
            Self::Italian { partita_iva, .. } => partita_iva
                .as_ref()
                .map(|partita| ("IT".to_string(), partita.to_string())),
            Self::EuVat { country, vat } => Some((country.code().to_string(), vat.to_string())),
            Self::Foreign { country, id } => Some((country.to_string(), id.to_string())),
        }
    }

    /// The `CodiceFiscale` element of the Italian identity, when present.
    #[must_use]
    pub const fn codice_fiscale(&self) -> Option<&CodiceFiscale> {
        match self {
            Self::Italian { codice_fiscale, .. } => codice_fiscale.as_ref(),
            Self::EuVat { .. } | Self::Foreign { .. } => None,
        }
    }

    /// Whether the identity is the Italian form.
    #[must_use]
    pub const fn is_italian(&self) -> bool {
        matches!(self, Self::Italian { .. })
    }

    /// Whether the identity carries a `partita IVA`.
    #[must_use]
    pub const fn has_partita_iva(&self) -> bool {
        matches!(
            self,
            Self::Italian {
                partita_iva: Some(..),
                ..
            }
        )
    }
}

/// The seat of a party, one of the two forms the `Sede` admits.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Address {
    /// The Italian seat, with its `CAP` and `Provincia`.
    italian: Option<(Indirizzo, Cap, Comune, Provincia)>,
    /// The foreign seat, which writes the `CAP` `00000`, omits the
    /// `Provincia` and puts the foreign postal code in the `Indirizzo`.
    foreign: Option<(Indirizzo, Option<PostalCode>, Comune, Country)>,
}

impl Address {
    /// The seat in Italy, with its `Indirizzo`, `CAP`, `Comune` and
    /// `Provincia`.
    #[must_use]
    pub const fn italian(
        indirizzo: Indirizzo,
        cap: Cap,
        comune: Comune,
        provincia: Provincia,
    ) -> Self {
        Self {
            italian: Some((indirizzo, cap, comune, provincia)),
            foreign: None,
        }
    }

    /// The seat in Italy, each value converted.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments.
    pub fn try_italian<I, C, M>(
        indirizzo: I,
        cap: C,
        comune: M,
        provincia: Provincia,
    ) -> Result<Self, IdError>
    where
        Indirizzo: TryFrom<I>,
        <Indirizzo as TryFrom<I>>::Error: Into<IdError>,
        Cap: TryFrom<C>,
        <Cap as TryFrom<C>>::Error: Into<IdError>,
        Comune: TryFrom<M>,
        <Comune as TryFrom<M>>::Error: Into<IdError>,
    {
        Ok(Self::italian(
            Indirizzo::try_from(indirizzo).map_err(Into::into)?,
            Cap::try_from(cap).map_err(Into::into)?,
            Comune::try_from(comune).map_err(Into::into)?,
            provincia,
        ))
    }

    /// The seat abroad, which writes the `CAP` `00000`, omits the
    /// `Provincia` and puts the foreign postal code in the `Indirizzo`
    /// when present.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the `Indirizzo` exceeds 60 characters once the
    /// postal code is appended.
    pub fn foreign(
        indirizzo: Indirizzo,
        postal_code: Option<PostalCode>,
        comune: Comune,
        country: Country,
    ) -> Result<Self, IdError> {
        Self::foreign_of(indirizzo, postal_code, comune, country)
    }

    /// The seat abroad, each value converted.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments, or
    /// when the `Indirizzo` exceeds 60 characters once the postal code is
    /// appended.
    pub fn try_foreign<I, P, C>(
        indirizzo: I,
        postal_code: Option<P>,
        comune: C,
        country: Country,
    ) -> Result<Self, IdError>
    where
        Indirizzo: TryFrom<I>,
        <Indirizzo as TryFrom<I>>::Error: Into<IdError>,
        PostalCode: TryFrom<P>,
        <PostalCode as TryFrom<P>>::Error: Into<IdError>,
        Comune: TryFrom<C>,
        <Comune as TryFrom<C>>::Error: Into<IdError>,
    {
        let postal_code = convert_opt(postal_code)?;
        Self::foreign_of(
            Indirizzo::try_from(indirizzo).map_err(Into::into)?,
            postal_code,
            Comune::try_from(comune).map_err(Into::into)?,
            country,
        )
    }

    /// The check of the `Indirizzo` once the foreign postal code is
    /// appended.
    fn foreign_of(
        indirizzo: Indirizzo,
        postal_code: Option<PostalCode>,
        comune: Comune,
        country: Country,
    ) -> Result<Self, IdError> {
        let written = postal_code.as_ref().map_or_else(
            || indirizzo.as_str().to_string(),
            |postal| format!("{} {}", indirizzo.as_str(), postal.as_str()),
        );
        if written.chars().count() <= 60 {
            Ok(Self {
                italian: None,
                foreign: Some((indirizzo, postal_code, comune, country)),
            })
        } else {
            Err(IdError::Address(written))
        }
    }

    /// The `Indirizzo` to write, with the foreign postal code appended
    /// when present.
    #[must_use]
    pub fn indirizzo(&self) -> String {
        match &self.italian {
            Some((indirizzo, _, _, _)) => indirizzo.as_str().to_string(),
            None => match &self.foreign {
                Some((indirizzo, postal_code, _, _)) => postal_code.as_ref().map_or_else(
                    || indirizzo.as_str().to_string(),
                    |postal| format!("{} {}", indirizzo.as_str(), postal.as_str()),
                ),
                None => String::new(),
            },
        }
    }

    /// The `CAP` to write, `00000` for a foreign seat.
    #[must_use]
    pub fn cap(&self) -> &str {
        match &self.italian {
            Some((_, cap, _, _)) => cap.as_str(),
            None => Cap::FOREIGN.as_str(),
        }
    }

    /// The `Comune` to write.
    #[must_use]
    pub fn comune(&self) -> &str {
        match &self.italian {
            Some((_, _, comune, _)) => comune.as_str(),
            None => match &self.foreign {
                Some((_, _, comune, _)) => comune.as_str(),
                None => "",
            },
        }
    }

    /// The `Provincia` code of an Italian seat.
    #[must_use]
    pub const fn provincia_code(&self) -> Option<&str> {
        match &self.italian {
            Some((_, _, _, provincia)) => Some(provincia.code()),
            None => None,
        }
    }

    /// The `Nazione` to write, `ISO 3166-1` alpha-2.
    #[must_use]
    pub fn country_code(&self) -> String {
        match &self.foreign {
            Some((_, _, _, country)) => country.to_string(),
            None => "IT".to_string(),
        }
    }
}

/// The name of a party, one of the two forms of the `Anagrafica`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Name {
    /// The `Denominazione` of a business.
    Denomination(Denomination),
    /// The `Nome` and `Cognome` of a person.
    FirstLast {
        /// The `Nome`.
        first: PersonName,
        /// The `Cognome`.
        last: PersonName,
    },
}

/// A party of the header, filling the `CedentePrestatore` or the
/// `CessionarioCommittente`.
///
/// Constructed through one of its roles, the `Seller`, the `Buyer`, the
/// `Business` or the `Company`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Party {
    /// The `Anagrafica` name.
    name: Name,
    /// The tax identity, filling `IdFiscaleIVA` and `CodiceFiscale`.
    identity: TaxIdentity,
    /// The `Sede`.
    address: Address,
}

impl Party {
    /// The `Anagrafica` name.
    #[must_use]
    pub const fn name(&self) -> &Name {
        &self.name
    }

    /// The tax identity.
    #[must_use]
    pub const fn identity(&self) -> &TaxIdentity {
        &self.identity
    }

    /// The seat.
    #[must_use]
    pub const fn address(&self) -> &Address {
        &self.address
    }

    /// The `Anagrafica` element.
    fn anagrafica(&self) -> AnagraficaType {
        let content = match &self.name {
            Name::Denomination(denominazione) => {
                AnagraficaContent60TypeContent::Content61(AnagraficaContent61Type {
                    content: AnagraficaContent61TypeContent {
                        denominazione: denominazione.as_str().to_string(),
                    },
                })
            }
            Name::FirstLast { first, last } => {
                AnagraficaContent60TypeContent::Content62(AnagraficaContent62Type {
                    content: AnagraficaContent62TypeContent {
                        nome: first.as_str().to_string(),
                        cognome: last.as_str().to_string(),
                    },
                })
            }
        };
        AnagraficaType {
            content: AnagraficaTypeContent {
                content_60: AnagraficaContent60Type { content },
                titolo: None,
                cod_eori: None,
            },
        }
    }

    /// The `Sede` element.
    fn sede(&self) -> IndirizzoType {
        IndirizzoType {
            content: IndirizzoTypeContent {
                indirizzo: self.address.indirizzo(),
                numero_civico: None,
                cap: self.address.cap().to_string(),
                comune: self.address.comune().to_string(),
                provincia: self.address.provincia_code().map(str::to_string),
                nazione: self.address.country_code(),
            },
        }
    }

    /// The `IdFiscaleIVA` element, when the identity writes it.
    fn id_fiscale_iva(&self) -> Option<IdFiscaleType> {
        self.identity
            .id_fiscale_iva()
            .map(|(id_paese, id_codice)| IdFiscaleType {
                content: IdFiscaleTypeContent {
                    id_paese,
                    id_codice,
                },
            })
    }

    /// The `CedentePrestatore` element with its `RegimeFiscale`.
    pub(crate) fn cedente(&self, regime: RegimeFiscale) -> Option<CedentePrestatoreType> {
        let id_fiscale_iva = self.id_fiscale_iva()?;
        Some(CedentePrestatoreType {
            content: CedentePrestatoreTypeContent {
                dati_anagrafici: DatiAnagraficiCedenteType {
                    content: DatiAnagraficiCedenteTypeContent {
                        id_fiscale_iva,
                        codice_fiscale: self.identity.codice_fiscale().map(ToString::to_string),
                        anagrafica: self.anagrafica(),
                        albo_professionale: None,
                        provincia_albo: None,
                        numero_iscrizione_albo: None,
                        data_iscrizione_albo: None,
                        regime_fiscale: regime.generated(),
                    },
                },
                sede: self.sede(),
                stabile_organizzazione: None,
                iscrizione_rea: None,
                contatti: None,
                riferimento_amministrazione: None,
            },
        })
    }

    /// The `CessionarioCommittente` element.
    pub(crate) fn cessionario(&self) -> CessionarioCommittenteType {
        CessionarioCommittenteType {
            content: CessionarioCommittenteTypeContent {
                dati_anagrafici: DatiAnagraficiCessionarioType {
                    content: DatiAnagraficiCessionarioTypeContent {
                        id_fiscale_iva: self.id_fiscale_iva(),
                        codice_fiscale: self.identity.codice_fiscale().map(ToString::to_string),
                        anagrafica: self.anagrafica(),
                    },
                },
                sede: self.sede(),
                stabile_organizzazione: None,
                rappresentante_fiscale: None,
            },
        }
    }
}

/// The seller of a `FatturaElettronica`, an Italian VAT-registered party
/// carrying the `RegimeFiscale` of the `CedentePrestatore`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Seller {
    /// The party of the `CedentePrestatore`.
    party: Party,
    /// The `RegimeFiscale`.
    regime: RegimeFiscale,
}

impl Seller {
    /// An Italian company seller, its `Denominazione`, `partita IVA`,
    /// seat and `RegimeFiscale`.
    #[must_use]
    pub const fn denomination(
        denominazione: Denomination,
        partita_iva: PartitaIva,
        address: Address,
        regime: RegimeFiscale,
    ) -> Self {
        Self {
            party: Party {
                name: Name::Denomination(denominazione),
                identity: TaxIdentity::Italian {
                    partita_iva: Some(partita_iva),
                    codice_fiscale: None,
                },
                address,
            },
            regime,
        }
    }

    /// An Italian company seller, each value converted.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments.
    pub fn try_denomination<D, P, R>(
        denominazione: D,
        partita_iva: P,
        address: Address,
        regime: R,
    ) -> Result<Self, IdError>
    where
        Denomination: TryFrom<D>,
        <Denomination as TryFrom<D>>::Error: Into<IdError>,
        PartitaIva: TryFrom<P>,
        <PartitaIva as TryFrom<P>>::Error: Into<IdError>,
        RegimeFiscale: TryFrom<R>,
        <RegimeFiscale as TryFrom<R>>::Error: Into<IdError>,
    {
        Ok(Self::denomination(
            Denomination::try_from(denominazione).map_err(Into::into)?,
            PartitaIva::try_from(partita_iva).map_err(Into::into)?,
            address,
            RegimeFiscale::try_from(regime).map_err(Into::into)?,
        ))
    }

    /// An Italian sole-trader seller, its `Nome`, `Cognome`, `partita
    /// IVA`, `CodiceFiscale`, seat and `RegimeFiscale`.
    #[must_use]
    pub const fn sole_trader(
        first: PersonName,
        last: PersonName,
        partita_iva: PartitaIva,
        codice_fiscale: CodiceFiscale,
        address: Address,
        regime: RegimeFiscale,
    ) -> Self {
        Self {
            party: Party {
                name: Name::FirstLast { first, last },
                identity: TaxIdentity::Italian {
                    partita_iva: Some(partita_iva),
                    codice_fiscale: Some(codice_fiscale),
                },
                address,
            },
            regime,
        }
    }

    /// The party of the `CedentePrestatore`.
    #[must_use]
    pub const fn party(&self) -> &Party {
        &self.party
    }

    /// An Italian sole-trader seller, each value converted.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments.
    pub fn try_sole_trader<F, L, P, C, R>(
        first: F,
        last: L,
        partita_iva: P,
        codice_fiscale: C,
        address: Address,
        regime: R,
    ) -> Result<Self, IdError>
    where
        PersonName: TryFrom<F> + TryFrom<L>,
        <PersonName as TryFrom<F>>::Error: Into<IdError>,
        <PersonName as TryFrom<L>>::Error: Into<IdError>,
        PartitaIva: TryFrom<P>,
        <PartitaIva as TryFrom<P>>::Error: Into<IdError>,
        CodiceFiscale: TryFrom<C>,
        <CodiceFiscale as TryFrom<C>>::Error: Into<IdError>,
        RegimeFiscale: TryFrom<R>,
        <RegimeFiscale as TryFrom<R>>::Error: Into<IdError>,
    {
        Ok(Self::sole_trader(
            PersonName::try_from(first).map_err(Into::into)?,
            PersonName::try_from(last).map_err(Into::into)?,
            PartitaIva::try_from(partita_iva).map_err(Into::into)?,
            CodiceFiscale::try_from(codice_fiscale).map_err(Into::into)?,
            address,
            RegimeFiscale::try_from(regime).map_err(Into::into)?,
        ))
    }

    /// The `RegimeFiscale` of the seller.
    #[must_use]
    pub const fn regime(&self) -> &RegimeFiscale {
        &self.regime
    }

    /// The `CedentePrestatore` element of the seller, which always carries
    /// its `IdFiscaleIVA`, the seller being a VAT-registered Italian
    /// party.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "the seller carries its partita IVA by construction"
    )]
    pub(crate) fn cedente(&self) -> CedentePrestatoreType {
        self.party
            .cedente(self.regime)
            .expect("the seller carries its IdFiscaleIVA")
    }

    /// The `IdTrasmittente` pair of the seller, which transmits the
    /// ordinary invoices it sells.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "the seller carries its partita IVA by construction"
    )]
    pub(crate) fn id_trasmittente(&self) -> (String, String) {
        self.party
            .identity()
            .id_fiscale_iva()
            .expect("the seller carries its IdFiscaleIVA")
    }
}

impl From<Seller> for Party {
    fn from(seller: Seller) -> Self {
        seller.party
    }
}

/// The buyer of a domestic `FatturaElettronica`, an Italian party.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Buyer {
    /// The party of the `CessionarioCommittente`.
    party: Party,
}

impl Buyer {
    /// An Italian company buyer, its `Denominazione`, `partita IVA` and
    /// seat.
    #[must_use]
    pub const fn denomination(
        denominazione: Denomination,
        partita_iva: PartitaIva,
        address: Address,
    ) -> Self {
        Self {
            party: Party {
                name: Name::Denomination(denominazione),
                identity: TaxIdentity::Italian {
                    partita_iva: Some(partita_iva),
                    codice_fiscale: None,
                },
                address,
            },
        }
    }

    /// An Italian sole-trader buyer, its `Nome`, `Cognome`, `partita
    /// IVA`, `CodiceFiscale` and seat.
    #[must_use]
    pub const fn sole_trader(
        first: PersonName,
        last: PersonName,
        partita_iva: PartitaIva,
        codice_fiscale: CodiceFiscale,
        address: Address,
    ) -> Self {
        Self {
            party: Party {
                name: Name::FirstLast { first, last },
                identity: TaxIdentity::Italian {
                    partita_iva: Some(partita_iva),
                    codice_fiscale: Some(codice_fiscale),
                },
                address,
            },
        }
    }

    /// An Italian consumer buyer, its `Nome`, `Cognome`, `CodiceFiscale`
    /// and seat.
    #[must_use]
    pub const fn consumer(
        first: PersonName,
        last: PersonName,
        codice_fiscale: CodiceFiscale,
        address: Address,
    ) -> Self {
        Self {
            party: Party {
                name: Name::FirstLast { first, last },
                identity: TaxIdentity::Italian {
                    partita_iva: None,
                    codice_fiscale: Some(codice_fiscale),
                },
                address,
            },
        }
    }

    /// An Italian company buyer, each value converted.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments.
    pub fn try_denomination<D, P>(
        denominazione: D,
        partita_iva: P,
        address: Address,
    ) -> Result<Self, IdError>
    where
        Denomination: TryFrom<D>,
        <Denomination as TryFrom<D>>::Error: Into<IdError>,
        PartitaIva: TryFrom<P>,
        <PartitaIva as TryFrom<P>>::Error: Into<IdError>,
    {
        Ok(Self::denomination(
            Denomination::try_from(denominazione).map_err(Into::into)?,
            PartitaIva::try_from(partita_iva).map_err(Into::into)?,
            address,
        ))
    }

    /// An Italian sole-trader buyer, each value converted.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments.
    pub fn try_sole_trader<F, L, P, C>(
        first: F,
        last: L,
        partita_iva: P,
        codice_fiscale: C,
        address: Address,
    ) -> Result<Self, IdError>
    where
        PersonName: TryFrom<F> + TryFrom<L>,
        <PersonName as TryFrom<F>>::Error: Into<IdError>,
        <PersonName as TryFrom<L>>::Error: Into<IdError>,
        PartitaIva: TryFrom<P>,
        <PartitaIva as TryFrom<P>>::Error: Into<IdError>,
        CodiceFiscale: TryFrom<C>,
        <CodiceFiscale as TryFrom<C>>::Error: Into<IdError>,
    {
        Ok(Self::sole_trader(
            PersonName::try_from(first).map_err(Into::into)?,
            PersonName::try_from(last).map_err(Into::into)?,
            PartitaIva::try_from(partita_iva).map_err(Into::into)?,
            CodiceFiscale::try_from(codice_fiscale).map_err(Into::into)?,
            address,
        ))
    }

    /// An Italian consumer buyer, each value converted.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments.
    pub fn try_consumer<F, L, C>(
        first: F,
        last: L,
        codice_fiscale: C,
        address: Address,
    ) -> Result<Self, IdError>
    where
        PersonName: TryFrom<F> + TryFrom<L>,
        <PersonName as TryFrom<F>>::Error: Into<IdError>,
        <PersonName as TryFrom<L>>::Error: Into<IdError>,
        CodiceFiscale: TryFrom<C>,
        <CodiceFiscale as TryFrom<C>>::Error: Into<IdError>,
    {
        Ok(Self::consumer(
            PersonName::try_from(first).map_err(Into::into)?,
            PersonName::try_from(last).map_err(Into::into)?,
            CodiceFiscale::try_from(codice_fiscale).map_err(Into::into)?,
            address,
        ))
    }

    /// The party of the `CessionarioCommittente`.
    #[must_use]
    pub const fn party(&self) -> &Party {
        &self.party
    }
}

impl From<Buyer> for Party {
    fn from(buyer: Buyer) -> Self {
        buyer.party
    }
}

/// A non-Italian business, the EU or non-EU client of the art. 7-ter
/// invoice and the supplier of the `TD17` and `TD19` self-invoice.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Business {
    /// The party of the document.
    party: Party,
}

impl Business {
    /// An EU business, its member state, `Denominazione`, VAT number and
    /// seat.
    #[must_use]
    pub const fn eu(
        country: EuCountry,
        denominazione: Denomination,
        vat: ForeignTaxId,
        address: Address,
    ) -> Self {
        Self {
            party: Party {
                name: Name::Denomination(denominazione),
                identity: TaxIdentity::EuVat { country, vat },
                address,
            },
        }
    }

    /// A non-EU business, its country, `Denominazione`, fiscal
    /// identifier of residence and seat.
    #[must_use]
    pub const fn foreign(
        country: Country,
        denominazione: Denomination,
        id: ForeignTaxId,
        address: Address,
    ) -> Self {
        Self {
            party: Party {
                name: Name::Denomination(denominazione),
                identity: TaxIdentity::Foreign { country, id },
                address,
            },
        }
    }

    /// An EU business, each value converted. The `country` is the member
    /// state the caller resolves to the `EuCountry` enum.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments.
    pub fn try_eu<D, V>(
        country: EuCountry,
        denominazione: D,
        vat: V,
        address: Address,
    ) -> Result<Self, IdError>
    where
        Denomination: TryFrom<D>,
        <Denomination as TryFrom<D>>::Error: Into<IdError>,
        ForeignTaxId: TryFrom<V>,
        <ForeignTaxId as TryFrom<V>>::Error: Into<IdError>,
    {
        Ok(Self::eu(
            country,
            Denomination::try_from(denominazione).map_err(Into::into)?,
            ForeignTaxId::try_from(vat).map_err(Into::into)?,
            address,
        ))
    }

    /// A non-EU business, each value converted. The `country` is the
    /// `ISO 3166-1` code the caller resolves to the `Country` enum.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments.
    pub fn try_foreign<D, I>(
        country: Country,
        denominazione: D,
        id: I,
        address: Address,
    ) -> Result<Self, IdError>
    where
        Denomination: TryFrom<D>,
        <Denomination as TryFrom<D>>::Error: Into<IdError>,
        ForeignTaxId: TryFrom<I>,
        <ForeignTaxId as TryFrom<I>>::Error: Into<IdError>,
    {
        Ok(Self::foreign(
            country,
            Denomination::try_from(denominazione).map_err(Into::into)?,
            ForeignTaxId::try_from(id).map_err(Into::into)?,
            address,
        ))
    }

    /// The party of the document.
    #[must_use]
    pub const fn party(&self) -> &Party {
        &self.party
    }

    /// The `CedentePrestatore` element of the supplier of a self-invoice,
    /// which always carries its `IdFiscaleIVA`, the supplier being a
    /// non-Italian business.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "the foreign supplier carries its fiscal identifier by construction"
    )]
    pub(crate) fn cedente(&self, regime: RegimeFiscale) -> CedentePrestatoreType {
        self.party
            .cedente(regime)
            .expect("the foreign supplier carries its IdFiscaleIVA")
    }
}

impl From<Business> for Party {
    fn from(business: Business) -> Self {
        business.party
    }
}

/// An Italian company with its `partita IVA`, the buyer of the `TD17` and
/// `TD19` self-invoice.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Company {
    /// The party of the `CessionarioCommittente`.
    party: Party,
}

impl Company {
    /// An Italian company, its `Denominazione`, `partita IVA` and seat.
    #[must_use]
    pub const fn denomination(
        denominazione: Denomination,
        partita_iva: PartitaIva,
        address: Address,
    ) -> Self {
        Self {
            party: Party {
                name: Name::Denomination(denominazione),
                identity: TaxIdentity::Italian {
                    partita_iva: Some(partita_iva),
                    codice_fiscale: None,
                },
                address,
            },
        }
    }

    /// An Italian sole trader, its `Nome`, `Cognome`, `partita IVA`,
    /// `CodiceFiscale` and seat.
    #[must_use]
    pub const fn sole_trader(
        first: PersonName,
        last: PersonName,
        partita_iva: PartitaIva,
        codice_fiscale: CodiceFiscale,
        address: Address,
    ) -> Self {
        Self {
            party: Party {
                name: Name::FirstLast { first, last },
                identity: TaxIdentity::Italian {
                    partita_iva: Some(partita_iva),
                    codice_fiscale: Some(codice_fiscale),
                },
                address,
            },
        }
    }

    /// An Italian company, each value converted.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments.
    pub fn try_denomination<D, P>(
        denominazione: D,
        partita_iva: P,
        address: Address,
    ) -> Result<Self, IdError>
    where
        Denomination: TryFrom<D>,
        <Denomination as TryFrom<D>>::Error: Into<IdError>,
        PartitaIva: TryFrom<P>,
        <PartitaIva as TryFrom<P>>::Error: Into<IdError>,
    {
        Ok(Self::denomination(
            Denomination::try_from(denominazione).map_err(Into::into)?,
            PartitaIva::try_from(partita_iva).map_err(Into::into)?,
            address,
        ))
    }

    /// An Italian sole trader, each value converted.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the first conversion error of the arguments.
    pub fn try_sole_trader<F, L, P, C>(
        first: F,
        last: L,
        partita_iva: P,
        codice_fiscale: C,
        address: Address,
    ) -> Result<Self, IdError>
    where
        PersonName: TryFrom<F> + TryFrom<L>,
        <PersonName as TryFrom<F>>::Error: Into<IdError>,
        <PersonName as TryFrom<L>>::Error: Into<IdError>,
        PartitaIva: TryFrom<P>,
        <PartitaIva as TryFrom<P>>::Error: Into<IdError>,
        CodiceFiscale: TryFrom<C>,
        <CodiceFiscale as TryFrom<C>>::Error: Into<IdError>,
    {
        Ok(Self::sole_trader(
            PersonName::try_from(first).map_err(Into::into)?,
            PersonName::try_from(last).map_err(Into::into)?,
            PartitaIva::try_from(partita_iva).map_err(Into::into)?,
            CodiceFiscale::try_from(codice_fiscale).map_err(Into::into)?,
            address,
        ))
    }

    /// The party of the `CessionarioCommittente`.
    #[must_use]
    pub const fn party(&self) -> &Party {
        &self.party
    }

    /// The `IdTrasmittente` pair of the company, which transmits the
    /// self-invoices it integrates as the Italian buyer.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "the company carries its partita IVA by construction"
    )]
    pub(crate) fn id_trasmittente(&self) -> (String, String) {
        self.party
            .identity()
            .id_fiscale_iva()
            .expect("the company carries its IdFiscaleIVA")
    }
}

impl From<Company> for Party {
    fn from(company: Company) -> Self {
        company.party
    }
}

/// The serialised form of an [`Address`], the two seats as tagged variants.
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
enum AddressForm {
    /// The Italian seat, with its `CAP` and `Provincia`.
    Italian {
        /// The `Indirizzo`.
        indirizzo: Indirizzo,
        /// The `CAP`.
        cap: Cap,
        /// The `Comune`.
        comune: Comune,
        /// The `Provincia`.
        provincia: Provincia,
    },
    /// The foreign seat, which writes the `CAP` `00000`.
    Foreign {
        /// The `Indirizzo`.
        indirizzo: Indirizzo,
        /// The foreign postal code, appended to the `Indirizzo`.
        postal_code: Option<PostalCode>,
        /// The `Comune`.
        comune: Comune,
        /// The country of the seat.
        country: Country,
    },
}

#[cfg(feature = "serde")]
impl serde::Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let form = if let Some((indirizzo, cap, comune, provincia)) = &self.italian {
            AddressForm::Italian {
                indirizzo: indirizzo.clone(),
                cap: *cap,
                comune: comune.clone(),
                provincia: *provincia,
            }
        } else if let Some((indirizzo, postal_code, comune, country)) = &self.foreign {
            AddressForm::Foreign {
                indirizzo: indirizzo.clone(),
                postal_code: postal_code.clone(),
                comune: comune.clone(),
                country: *country,
            }
        } else {
            return Err(serde::ser::Error::custom("the address carries no seat"));
        };
        form.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match AddressForm::deserialize(deserializer)? {
            AddressForm::Italian {
                indirizzo,
                cap,
                comune,
                provincia,
            } => Ok(Self::italian(indirizzo, cap, comune, provincia)),
            AddressForm::Foreign {
                indirizzo,
                postal_code,
                comune,
                country,
            } => Self::foreign(indirizzo, postal_code, comune, country)
                .map_err(serde::de::Error::custom),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TaxIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        enum Raw {
            /// The Italian identity, at least one identifier present.
            Italian {
                /// The `partita IVA`, when present.
                partita_iva: Option<PartitaIva>,
                /// The `CodiceFiscale`, when present.
                codice_fiscale: Option<CodiceFiscale>,
            },
            /// The EU business identity.
            EuVat {
                /// The member state.
                country: EuCountry,
                /// The VAT number.
                vat: ForeignTaxId,
            },
            /// The non-EU identity.
            Foreign {
                /// The country of residence.
                country: Country,
                /// The fiscal identifier of residence.
                id: ForeignTaxId,
            },
        }
        match Raw::deserialize(deserializer)? {
            Raw::Italian {
                partita_iva,
                codice_fiscale,
            } => {
                if partita_iva.is_some() || codice_fiscale.is_some() {
                    Ok(Self::Italian {
                        partita_iva,
                        codice_fiscale,
                    })
                } else {
                    Err(serde::de::Error::custom(
                        "the Italian identity needs at least one identifier",
                    ))
                }
            }
            Raw::EuVat { country, vat } => Ok(Self::EuVat { country, vat }),
            Raw::Foreign { country, id } => Ok(Self::Foreign { country, id }),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Seller {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Raw {
            /// The party of the `CedentePrestatore`.
            party: Party,
            /// The `RegimeFiscale`.
            regime: RegimeFiscale,
        }
        let Raw { party, regime } = Raw::deserialize(deserializer)?;
        let identity = party.identity();
        if identity.is_italian() && identity.has_partita_iva() {
            Ok(Self { party, regime })
        } else {
            Err(serde::de::Error::custom(
                "the seller must be an Italian party with its partita IVA",
            ))
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Buyer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Raw {
            /// The party of the `CessionarioCommittente`.
            party: Party,
        }
        let Raw { party } = Raw::deserialize(deserializer)?;
        if party.identity().is_italian() {
            Ok(Self { party })
        } else {
            Err(serde::de::Error::custom(
                "the buyer must be an Italian party",
            ))
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Business {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Raw {
            /// The party of the document.
            party: Party,
        }
        let Raw { party } = Raw::deserialize(deserializer)?;
        if !party.identity().is_italian() && matches!(party.name(), Name::Denomination(_)) {
            Ok(Self { party })
        } else {
            Err(serde::de::Error::custom(
                "the business must be a non-Italian party with its denomination",
            ))
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Company {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Raw {
            /// The party of the `CessionarioCommittente`.
            party: Party,
        }
        let Raw { party } = Raw::deserialize(deserializer)?;
        let identity = party.identity();
        if identity.is_italian() && identity.has_partita_iva() {
            Ok(Self { party })
        } else {
            Err(serde::de::Error::custom(
                "the company must be an Italian party with its partita IVA",
            ))
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing case"
)]
mod tests {
    use super::*;
    use crate::ids::PartitaIva;

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

    fn rome() -> Address {
        Address::italian(
            "VIA ROMA 1".parse().expect("a valid address"),
            "00100".parse().expect("a valid CAP"),
            "ROMA".parse().expect("a valid comune"),
            Provincia::Rm,
        )
    }

    #[test]
    fn seller_fills_the_header_elements() {
        let seller = Seller::denomination(
            "ALPHA SRL".parse().expect("a valid denomination"),
            alpha_piva(),
            rome(),
            RegimeFiscale::Rf01,
        );
        let cedente = seller.cedente();
        assert_eq!(
            cedente
                .content
                .dati_anagrafici
                .content
                .id_fiscale_iva
                .content
                .id_paese,
            "IT"
        );
        assert_eq!(
            cedente
                .content
                .dati_anagrafici
                .content
                .id_fiscale_iva
                .content
                .id_codice,
            "00012345674"
        );
        assert_eq!(
            cedente.content.dati_anagrafici.content.regime_fiscale,
            crate::generated::fpr12::RegimeFiscaleType::Rf01
        );
        assert_eq!(cedente.content.sede.content.nazione, "IT");
        assert_eq!(cedente.content.sede.content.cap, "00100");
        assert_eq!(
            cedente.content.sede.content.provincia.as_deref(),
            Some("RM")
        );
    }

    #[test]
    fn foreign_seat_writes_the_generic_cap() {
        let business = Business::foreign(
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
        let cessionario = business.party().cessionario();
        assert_eq!(
            cessionario.content.sede.content.indirizzo,
            "FIFTH AVENUE 1 10001"
        );
        assert_eq!(cessionario.content.sede.content.cap, Cap::FOREIGN.as_str());
        assert_eq!(cessionario.content.sede.content.provincia, None);
        assert_eq!(cessionario.content.sede.content.nazione, "US");
        assert_eq!(
            cessionario
                .content
                .dati_anagrafici
                .content
                .id_fiscale_iva
                .expect("the identity carries its IdFiscaleIVA")
                .content
                .id_paese,
            "US"
        );
    }

    #[test]
    fn consumer_carries_only_the_codice_fiscale() {
        let consumer = Buyer::consumer(
            "Mario".parse().expect("a valid name"),
            "Rossi".parse().expect("a valid name"),
            "MRALNE80E05H501C".parse().expect("a valid codice fiscale"),
            rome(),
        );
        let cessionario = consumer.party().cessionario();
        assert_eq!(
            cessionario.content.dati_anagrafici.content.id_fiscale_iva,
            None
        );
        assert_eq!(
            cessionario
                .content
                .dati_anagrafici
                .content
                .codice_fiscale
                .as_deref(),
            Some("MRALNE80E05H501C")
        );
        assert!(matches!(
            cessionario
                .content
                .dati_anagrafici
                .content
                .anagrafica
                .content
                .content_60
                .content,
            AnagraficaContent60TypeContent::Content62(_)
        ));
    }

    #[test]
    fn the_company_carries_the_partita_iva() {
        let company = Company::denomination(
            "BETA SRL".parse().expect("a valid denomination"),
            beta_piva(),
            Address::italian(
                "VIA MILANO 2".parse().expect("a valid address"),
                "20100".parse().expect("a valid CAP"),
                "MILANO".parse().expect("a valid comune"),
                Provincia::Mi,
            ),
        );
        assert!(company.party().identity().has_partita_iva());
        let cessionario = company.party().cessionario();
        assert_eq!(
            cessionario
                .content
                .dati_anagrafici
                .content
                .id_fiscale_iva
                .as_ref()
                .expect("the identity carries its IdFiscaleIVA")
                .content
                .id_codice,
            "00076543214"
        );
    }

    #[test]
    fn the_foreign_seat_checks_the_combined_length() {
        let long = "A".repeat(55);
        assert!(
            Address::foreign(
                long.parse().expect("a valid address"),
                Some("12345".parse().expect("a valid postal code")),
                "NEW YORK".parse().expect("a valid comune"),
                Country::US,
            )
            .is_err()
        );
        assert!(
            Address::foreign(
                long.parse().expect("a valid address"),
                None,
                "NEW YORK".parse().expect("a valid comune"),
                Country::US,
            )
            .is_ok()
        );
    }

    /// A foreign seat without a postal code writes the bare `Indirizzo`
    /// and the `Comune`, with the generic foreign `CAP`.
    #[test]
    fn the_foreign_seat_without_a_postal_code_writes_the_bare_indirizzo() {
        let seat = Address::foreign(
            "FIFTH AVENUE 1".parse().expect("a valid address"),
            None,
            "NEW YORK".parse().expect("a valid comune"),
            Country::US,
        )
        .expect("the seat is valid");
        assert_eq!(seat.indirizzo(), "FIFTH AVENUE 1");
        assert_eq!(seat.comune(), "NEW YORK");
        assert_eq!(seat.cap(), "00000");
    }

    #[test]
    fn the_italian_identity_needs_one_identifier() {
        assert!(TaxIdentity::try_italian::<&str, &str>(None, None).is_err());
        assert!(TaxIdentity::try_italian(Some(alpha_piva().to_string()), None::<&str>).is_ok());
    }

    /// A synthetic codice fiscale, never a real person.
    fn codice_fiscale() -> CodiceFiscale {
        "MRALNE80E05H501C"
            .parse()
            .expect("a synthetic valid codice fiscale")
    }

    /// The identity queries answer per form, for the Italian company,
    /// the Italian consumer, the EU business and the foreign party.
    #[test]
    fn the_identity_forms_answer_the_form_queries() {
        let company = TaxIdentity::try_italian(Some(alpha_piva()), Some(codice_fiscale()))
            .expect("an Italian company carries its identifiers");
        assert!(company.is_italian());
        assert!(company.has_partita_iva());

        let consumer = TaxIdentity::try_italian(None::<&str>, Some(codice_fiscale().to_string()))
            .expect("an Italian consumer carries its codice fiscale");
        assert!(consumer.is_italian());
        assert!(!consumer.has_partita_iva());

        let eu_business = TaxIdentity::eu_vat(
            EuCountry::De,
            "DE998877665".parse().expect("a valid identifier"),
        );
        assert!(!eu_business.is_italian());
        assert!(!eu_business.has_partita_iva());

        let foreign = TaxIdentity::foreign(
            Country::US,
            "US123456789".parse().expect("a valid identifier"),
        );
        assert!(!foreign.is_italian());
        assert!(!foreign.has_partita_iva());
    }

    /// The foreign seat of the New York client, never a real address.
    fn ny() -> Address {
        Address::foreign(
            "FIFTH AVENUE 1".parse().expect("a valid address"),
            Some("10001".parse().expect("a valid postal code")),
            "NEW YORK".parse().expect("a valid comune"),
            Country::US,
        )
        .expect("a valid foreign seat")
    }

    /// The generic `try_*` constructors of the parties build, from the
    /// string arguments, the same party that the typed constructor
    /// builds.
    #[expect(
        clippy::too_many_lines,
        reason = "one assertion per party, every typed field equal to the generic build"
    )]
    #[test]
    fn the_try_constructors_build_the_typed_parties() {
        assert_eq!(
            TaxIdentity::try_italian(Some("00012345674"), Some("MRALNE80E05H501C")),
            TaxIdentity::try_italian(Some(alpha_piva()), Some(codice_fiscale())),
        );
        assert_eq!(
            TaxIdentity::try_eu_vat(EuCountry::De, "DE998877665"),
            Ok(TaxIdentity::eu_vat(
                EuCountry::De,
                "DE998877665".parse().expect("a valid identifier")
            )),
        );
        assert_eq!(
            TaxIdentity::try_foreign(Country::US, "US123456789"),
            Ok(TaxIdentity::foreign(
                Country::US,
                "US123456789".parse().expect("a valid identifier")
            )),
        );
        assert_eq!(
            Address::try_italian("VIA ROMA 1", "00100", "ROMA", Provincia::Rm)
                .expect("the seat is valid"),
            Address::italian(
                "VIA ROMA 1".parse().expect("a valid address"),
                "00100".parse().expect("a valid CAP"),
                "ROMA".parse().expect("a valid comune"),
                Provincia::Rm
            ),
        );
        assert_eq!(
            Address::try_foreign("FIFTH AVENUE 1", Some("10001"), "NEW YORK", Country::US)
                .expect("the seat is valid"),
            ny(),
        );
        assert_eq!(
            Address::try_foreign("FIFTH AVENUE 1", None::<&str>, "NEW YORK", Country::US)
                .expect("the seat is valid"),
            Address::foreign(
                "FIFTH AVENUE 1".parse().expect("a valid address"),
                None,
                "NEW YORK".parse().expect("a valid comune"),
                Country::US,
            )
            .expect("the seat is valid"),
        );
        assert_eq!(
            Seller::try_denomination("ALPHA SRL", "00012345674", rome(), "RF01")
                .expect("the seller is valid"),
            Seller::denomination(
                "ALPHA SRL".parse().expect("a valid denomination"),
                alpha_piva(),
                rome(),
                RegimeFiscale::Rf01
            ),
        );
        assert_eq!(
            Seller::try_sole_trader(
                "Mario",
                "Rossi",
                "00012345674",
                "MRALNE80E05H501C",
                rome(),
                "RF01",
            )
            .expect("the seller is valid"),
            Seller::sole_trader(
                "Mario".parse().expect("a valid name"),
                "Rossi".parse().expect("a valid name"),
                alpha_piva(),
                codice_fiscale(),
                rome(),
                RegimeFiscale::Rf01
            ),
        );
        assert_eq!(
            Buyer::try_denomination("BETA SRL", "00076543214", rome()).expect("valid"),
            Buyer::denomination(
                "BETA SRL".parse().expect("a valid denomination"),
                beta_piva(),
                rome(),
            ),
        );
        assert_eq!(
            Buyer::try_sole_trader("Mario", "Rossi", "00012345674", "MRALNE80E05H501C", rome())
                .expect("the buyer is valid"),
            Buyer::sole_trader(
                "Mario".parse().expect("a valid name"),
                "Rossi".parse().expect("a valid name"),
                alpha_piva(),
                codice_fiscale(),
                rome(),
            ),
        );
        assert_eq!(
            Buyer::try_consumer("Mario", "Rossi", "MRALNE80E05H501C", rome())
                .expect("the buyer is valid"),
            Buyer::consumer(
                "Mario".parse().expect("a valid name"),
                "Rossi".parse().expect("a valid name"),
                codice_fiscale(),
                rome(),
            ),
        );
        assert_eq!(
            Business::try_eu(EuCountry::De, "MUSTER GMBH", "DE998877665", ny())
                .expect("the business is valid"),
            Business::eu(
                EuCountry::De,
                "MUSTER GMBH".parse().expect("a valid denomination"),
                "DE998877665".parse().expect("a valid identifier"),
                ny(),
            ),
        );
        assert_eq!(
            Business::try_foreign(Country::US, "ACME INC", "US123456789", ny())
                .expect("the business is valid"),
            Business::foreign(
                Country::US,
                "ACME INC".parse().expect("a valid denomination"),
                "US123456789".parse().expect("a valid identifier"),
                ny(),
            ),
        );
        assert_eq!(
            Company::try_denomination("BETA SRL", "00076543214", rome()).expect("valid"),
            Company::denomination(
                "BETA SRL".parse().expect("a valid denomination"),
                beta_piva(),
                rome(),
            ),
        );
        assert_eq!(
            Company::try_sole_trader("Mario", "Rossi", "00012345674", "MRALNE80E05H501C", rome())
                .expect("the company is valid"),
            Company::sole_trader(
                "Mario".parse().expect("a valid name"),
                "Rossi".parse().expect("a valid name"),
                alpha_piva(),
                codice_fiscale(),
                rome(),
            ),
        );
    }

    /// The generic `try_*` constructors of the parties report, for every
    /// argument position, the `IdError` of the identifier that fails.
    #[expect(
        clippy::too_many_lines,
        reason = "one assertion per argument position of every party constructor"
    )]
    #[test]
    fn the_try_constructors_report_each_failing_argument() {
        assert!(matches!(
            TaxIdentity::try_italian(Some("00012345675"), None::<&str>),
            Err(IdError::PartitaIva(..))
        ));
        assert!(matches!(
            TaxIdentity::try_italian(None::<&str>, Some("MRALNE80E05H501X")),
            Err(IdError::CodiceFiscale(..))
        ));
        assert!(matches!(
            TaxIdentity::try_eu_vat("XX", "DE998877665"),
            Err(IdError::EuCountry(..))
        ));
        assert!(matches!(
            TaxIdentity::try_eu_vat("DE", "xx"),
            Err(IdError::ForeignTaxId(..))
        ));
        assert!(matches!(
            TaxIdentity::try_foreign(Country::US, "xx"),
            Err(IdError::ForeignTaxId(..))
        ));
        assert!(matches!(
            Address::try_italian("A".repeat(61), "00100", "ROMA", Provincia::Rm),
            Err(IdError::Indirizzo(..))
        ));
        assert!(matches!(
            Address::try_italian("VIA ROMA 1", "0010", "ROMA", Provincia::Rm),
            Err(IdError::Cap(..))
        ));
        assert!(matches!(
            Address::try_italian("VIA ROMA 1", "00100", "A".repeat(61), Provincia::Rm),
            Err(IdError::Comune(..))
        ));
        assert!(matches!(
            Address::try_foreign("A".repeat(61), None::<&str>, "NEW YORK", Country::US),
            Err(IdError::Indirizzo(..))
        ));
        assert!(matches!(
            Address::try_foreign(
                "FIFTH AVENUE 1",
                Some("12345678901"),
                "NEW YORK",
                Country::US
            ),
            Err(IdError::PostalCode(..))
        ));
        assert!(matches!(
            Address::try_foreign("FIFTH AVENUE 1", None::<&str>, "A".repeat(61), Country::US),
            Err(IdError::Comune(..))
        ));
        assert!(matches!(
            Seller::try_denomination("A".repeat(81), "00012345674", rome(), RegimeFiscale::Rf01),
            Err(IdError::Denomination(..))
        ));
        assert!(matches!(
            Seller::try_denomination("ALPHA SRL", "00012345675", rome(), RegimeFiscale::Rf01),
            Err(IdError::PartitaIva(..))
        ));
        assert!(matches!(
            Seller::try_denomination("ALPHA SRL", "00012345674", rome(), "RX99"),
            Err(IdError::RegimeFiscale(..))
        ));
        assert!(matches!(
            Seller::try_sole_trader(
                "A".repeat(61),
                "Rossi",
                "00012345674",
                "MRALNE80E05H501C",
                rome(),
                RegimeFiscale::Rf01
            ),
            Err(IdError::PersonName(..))
        ));
        assert!(matches!(
            Seller::try_sole_trader(
                "Mario",
                "A".repeat(61),
                "00012345674",
                "MRALNE80E05H501C",
                rome(),
                RegimeFiscale::Rf01
            ),
            Err(IdError::PersonName(..))
        ));
        assert!(matches!(
            Seller::try_sole_trader(
                "Mario",
                "Rossi",
                "00012345675",
                "MRALNE80E05H501C",
                rome(),
                RegimeFiscale::Rf01
            ),
            Err(IdError::PartitaIva(..))
        ));
        assert!(matches!(
            Seller::try_sole_trader(
                "Mario",
                "Rossi",
                "00012345674",
                "MRALNE80E05H501X",
                rome(),
                RegimeFiscale::Rf01
            ),
            Err(IdError::CodiceFiscale(..))
        ));
        assert!(matches!(
            Seller::try_sole_trader(
                "Mario",
                "Rossi",
                "00012345674",
                "MRALNE80E05H501C",
                rome(),
                "RX99"
            ),
            Err(IdError::RegimeFiscale(..))
        ));
        assert!(matches!(
            Buyer::try_denomination("A".repeat(81), "00076543214", rome()),
            Err(IdError::Denomination(..))
        ));
        assert!(matches!(
            Buyer::try_denomination("BETA SRL", "00076543215", rome()),
            Err(IdError::PartitaIva(..))
        ));
        assert!(matches!(
            Buyer::try_sole_trader(
                "A".repeat(61),
                "Rossi",
                "00012345674",
                "MRALNE80E05H501C",
                rome()
            ),
            Err(IdError::PersonName(..))
        ));
        assert!(matches!(
            Buyer::try_sole_trader(
                "Mario",
                "A".repeat(61),
                "00012345674",
                "MRALNE80E05H501C",
                rome()
            ),
            Err(IdError::PersonName(..))
        ));
        assert!(matches!(
            Buyer::try_sole_trader("Mario", "Rossi", "00012345675", "MRALNE80E05H501C", rome()),
            Err(IdError::PartitaIva(..))
        ));
        assert!(matches!(
            Buyer::try_sole_trader("Mario", "Rossi", "00012345674", "MRALNE80E05H501X", rome()),
            Err(IdError::CodiceFiscale(..))
        ));
        assert!(matches!(
            Buyer::try_consumer("A".repeat(61), "Rossi", "MRALNE80E05H501C", rome()),
            Err(IdError::PersonName(..))
        ));
        assert!(matches!(
            Buyer::try_consumer("Mario", "A".repeat(61), "MRALNE80E05H501C", rome()),
            Err(IdError::PersonName(..))
        ));
        assert!(matches!(
            Buyer::try_consumer("Mario", "Rossi", "MRALNE80E05H501X", rome()),
            Err(IdError::CodiceFiscale(..))
        ));
        assert!(matches!(
            Business::try_eu(EuCountry::De, "A".repeat(81), "DE998877665", ny()),
            Err(IdError::Denomination(..))
        ));
        assert!(matches!(
            Business::try_eu(EuCountry::De, "MUSTER GMBH", "xx", ny()),
            Err(IdError::ForeignTaxId(..))
        ));
        assert!(matches!(
            Business::try_foreign(Country::US, "A".repeat(81), "US123456789", ny()),
            Err(IdError::Denomination(..))
        ));
        assert!(matches!(
            Business::try_foreign(Country::US, "ACME INC", "xx", ny()),
            Err(IdError::ForeignTaxId(..))
        ));
        assert!(matches!(
            Company::try_denomination("A".repeat(81), "00076543214", rome()),
            Err(IdError::Denomination(..))
        ));
        assert!(matches!(
            Company::try_denomination("BETA SRL", "00076543215", rome()),
            Err(IdError::PartitaIva(..))
        ));
        assert!(matches!(
            Company::try_sole_trader(
                "A".repeat(61),
                "Rossi",
                "00012345674",
                "MRALNE80E05H501C",
                rome()
            ),
            Err(IdError::PersonName(..))
        ));
        assert!(matches!(
            Company::try_sole_trader(
                "Mario",
                "A".repeat(61),
                "00012345674",
                "MRALNE80E05H501C",
                rome()
            ),
            Err(IdError::PersonName(..))
        ));
        assert!(matches!(
            Company::try_sole_trader("Mario", "Rossi", "00012345675", "MRALNE80E05H501C", rome()),
            Err(IdError::PartitaIva(..))
        ));
        assert!(matches!(
            Company::try_sole_trader("Mario", "Rossi", "00012345674", "MRALNE80E05H501X", rome()),
            Err(IdError::CodiceFiscale(..))
        ));
    }
}
