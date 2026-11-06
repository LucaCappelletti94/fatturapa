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

/// The tax identity of a party, one of the three forms the header admits.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// present.
    ///
    /// # Errors
    ///
    /// Returns `Err` when both identifiers are absent.
    pub fn italian(
        partita_iva: Option<PartitaIva>,
        codice_fiscale: Option<CodiceFiscale>,
    ) -> Result<Self, IdError> {
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

    /// Builds the non-EU identity.
    #[must_use]
    pub const fn foreign(country: Country, id: ForeignTaxId) -> Self {
        Self::Foreign { country, id }
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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

    #[test]
    fn the_italian_identity_needs_one_identifier() {
        assert!(TaxIdentity::italian(None, None).is_err());
        assert!(TaxIdentity::italian(Some(alpha_piva()), None).is_ok());
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
        let company = TaxIdentity::italian(Some(alpha_piva()), Some(codice_fiscale()))
            .expect("an Italian company carries its identifiers");
        assert!(company.is_italian());
        assert!(company.has_partita_iva());

        let consumer = TaxIdentity::italian(None, Some(codice_fiscale()))
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
}
