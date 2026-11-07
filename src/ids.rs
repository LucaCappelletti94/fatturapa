//! The typed identifiers and text of the `FatturaPA` elements, each
//! validated on construction.
//!
//! Every newtype carries a private field, exposes `FromStr`, `Display` and
//! `TryFrom<&str>`, and rejects the value when it does not fit the element
//! it fills. The check digit of `PartitaIva` follows the Luhn rule of DM
//! 23/12/1976, the check character of `CodiceFiscale` follows the value
//! tables of D.P.R. 29/9/1973 n. 605, `Cap` admits the `00000` the Agenzia
//! prescribes for a foreign address, and the text newtypes carry the length
//! and character set of the `StringLatin` types of the XSD.

use crate::generated::fpr12::{NaturaType, RegimeFiscaleType};
use iban::IbanLike;
use rust_decimal::Decimal;
use std::fmt;
use std::str::FromStr;

/// The 1-based positions of the seven digits of a personal `CodiceFiscale`,
/// 0-based, where an omocodia letter may replace the digit.
const OMOCODIA_POSITIONS: [usize; 7] = [6, 7, 9, 10, 12, 13, 14];

/// The omocodia letters with the digit they replace, `0` to `9`.
const OMOCODIA: &[u8] = b"LMNPQRSTUV";

/// The value of each letter, `A` to `Z`, at an even position.
const EVEN_LETTER: [u32; 26] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
];

/// The value of each letter, `A` to `Z`, at an odd position.
const ODD_LETTER: [u32; 26] = [
    1, 0, 5, 7, 9, 13, 15, 17, 19, 21, 2, 4, 18, 20, 11, 3, 6, 8, 12, 14, 16, 10, 22, 25, 24, 23,
];

/// The `Display`, `FromStr`, `TryFrom<&str>` and `AsRef<str>` of a
/// newtype whose validating constructor is `try_new`. The `field` arm
/// writes the text of the private `String` field, the `accessor` arm the
/// text of the newtype's own `as_str`.
macro_rules! id_newtype {
    ($name:ident, field) => {
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::try_new(s)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = IdError;

            fn try_from(s: &str) -> Result<Self, Self::Error> {
                Self::from_str(s)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;

            fn try_from(s: String) -> Result<Self, Self::Error> {
                Self::from_str(&s)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
    ($name:ident, accessor) => {
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::try_new(s)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = IdError;

            fn try_from(s: &str) -> Result<Self, Self::Error> {
                Self::from_str(s)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;

            fn try_from(s: String) -> Result<Self, Self::Error> {
                Self::from_str(&s)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }
    };
}

/// Why an identifier does not fit the element it would fill.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IdError {
    /// `PartitaIva` is not 11 digits or fails its check digit.
    #[error("not a valid partita IVA: {0}")]
    PartitaIva(String),
    /// `CodiceFiscale` is not 16 or 11 characters or fails its checks.
    #[error("not a valid codice fiscale: {0}")]
    CodiceFiscale(String),
    /// `Cap` is not five digits.
    #[error("not a valid CAP, five digits: {0}")]
    Cap(String),
    /// `Provincia` is not a current Italian province code.
    #[error("not a current Italian province code: {0}")]
    Provincia(String),
    /// `CodiceDestinatario` is not six or seven uppercase alphanumeric
    /// characters.
    #[error("not a valid codice destinatario, six or seven characters: {0}")]
    CodiceDestinatario(String),
    /// `ProgressivoInvio` is not one to five alphanumeric characters.
    #[error("not a valid progressivo di invio, one to five alphanumeric characters: {0}")]
    ProgressivoInvio(String),
    /// `Bic` is not the eight or eleven character XSD pattern.
    #[error("not a valid BIC: {0}")]
    Bic(String),
    /// `ForeignTaxId` is not one to 28 uppercase alphanumeric characters.
    #[error("not a valid foreign tax identifier, one to 28 characters: {0}")]
    ForeignTaxId(String),
    /// `Telefono` is not five to 12 printable ASCII characters.
    #[error("not a valid telephone number, five to 12 characters: {0}")]
    Telefono(String),
    /// `Pec` is not an e-mail of at most 256 characters.
    #[error("not a valid PEC address: {0}")]
    Pec(String),
    /// `Email` is not an e-mail of seven to 256 characters.
    #[error("not a valid contact e-mail, seven to 256 characters: {0}")]
    Email(String),
    /// `Iban` fails the mod-97 check.
    #[error("not a valid IBAN: {0}")]
    Iban(String),
    /// `Aliquota` is not a percentage above zero and at most 100 with two
    /// decimals.
    #[error("not a valid VAT rate, above zero and at most 100 with two decimals: {0}")]
    Aliquota(String),
    /// `ExchangeRate` is not above zero or carries more than four
    /// decimals.
    #[error("not a valid exchange rate, above zero and at most four decimals: {0}")]
    ExchangeRate(String),
    /// `RiferimentoNormativo` is not one to 100 Latin characters.
    #[error("not a valid normative reference, one to 100 characters: {0}")]
    RiferimentoNormativo(String),
    /// `EuCountry` is not one of the 27 member states.
    #[error("not an EU member state: {0}")]
    EuCountry(String),
    /// `Natura` is not a current code.
    #[error("not a valid natura code: {0}")]
    Natura(String),
    /// A `Sconto` value is not a valid percentage or amount.
    #[error("not a valid discount or markup value: {0}")]
    Sconto(String),
    /// `Numero` is not one to 20 printable basic Latin characters with at
    /// least one digit.
    #[error("not a valid document number, one to 20 printable characters with a digit: {0}")]
    Numero(String),
    /// `IdDocumento` is not one to 20 printable basic Latin characters.
    #[error("not a valid document identifier, one to 20 printable characters: {0}")]
    IdDocumento(String),
    /// `Causale` is not one to 200 Latin characters.
    #[error("not a valid causale, one to 200 characters: {0}")]
    Causale(String),
    /// `Description` is not one to 1000 Latin characters.
    #[error("not a valid description, one to 1000 characters: {0}")]
    Description(String),
    /// `Denomination` is not one to 80 Latin characters.
    #[error("not a valid denomination, one to 80 characters: {0}")]
    Denomination(String),
    /// `PersonName` is not one to 60 Latin characters.
    #[error("not a valid name, one to 60 characters: {0}")]
    PersonName(String),
    /// `Indirizzo` is not one to 60 Latin characters.
    #[error("not a valid address, one to 60 characters: {0}")]
    Indirizzo(String),
    /// `Comune` is not one to 60 Latin characters.
    #[error("not a valid comune, one to 60 characters: {0}")]
    Comune(String),
    /// `UnitaMisura` is not one to 10 printable basic Latin characters.
    #[error("not a valid unit of measure, one to 10 printable characters: {0}")]
    UnitaMisura(String),
    /// `PostalCode` is not one to 10 uppercase alphanumeric characters,
    /// spaces or hyphens.
    #[error("not a valid postal code: {0}")]
    PostalCode(String),
    /// The `Indirizzo` of a foreign seat exceeds 60 characters once the
    /// postal code is appended.
    #[error("the foreign address exceeds 60 characters with its postal code: {0}")]
    Address(String),
    /// `RegimeFiscale` is not one of the codes of the XSD enumeration.
    #[error("not a valid regime fiscale code: {0}")]
    RegimeFiscale(String),
    /// The `Amount` is not a decimal number.
    #[error("not a valid amount, a decimal number: {0}")]
    Amount(String),
    /// The `Date` is not a `YYYY-MM-DD` date.
    #[error("not a valid date, the YYYY-MM-DD form: {0}")]
    Date(String),
    /// The `DateTime` is not an `xs:dateTime` value.
    #[error("not a valid date and time, the xs:dateTime form: {0}")]
    DateTime(String),
}

impl From<core::convert::Infallible> for IdError {
    fn from(value: core::convert::Infallible) -> Self {
        match value {}
    }
}

impl From<rust_decimal::Error> for IdError {
    fn from(error: rust_decimal::Error) -> Self {
        Self::Amount(error.to_string())
    }
}

/// The `IdCodice` of an Italian party, the 11-digit `partita IVA` with the
/// Luhn check digit of DM 23/12/1976.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PartitaIva(String);

impl PartitaIva {
    /// Builds the `PartitaIva` when `digits` is 11 digits with a valid
    /// check digit.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::PartitaIva`] when the value is not 11 digits
    /// with a valid check digit.
    pub fn try_new(digits: impl Into<String>) -> Result<Self, IdError> {
        let digits = digits.into();
        let all_zeros = digits.bytes().all(|b| b == b'0');
        if digits.len() == 11
            && !all_zeros
            && digits.bytes().all(|b| b.is_ascii_digit())
            && luhn_ok(digits.as_bytes())
        {
            Ok(Self(digits))
        } else {
            Err(IdError::PartitaIva(digits))
        }
    }

    /// The 11 digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(PartitaIva, field);

/// The `CodiceFiscale` of a party, one of the two forms the SDI admits.
///
/// The 16-character form of a natural person carries its check character
/// and the omocodia letters `LMNPQRSTUV` in the digit positions, per D.P.R.
/// 29/9/1973 n. 605. The 11-digit form of a non-natural person is the
/// `partita IVA` with its check digit.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CodiceFiscale(String);

impl CodiceFiscale {
    /// Builds the `CodiceFiscale` when the value fits one of the two forms.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::CodiceFiscale`] when the value fits neither of
    /// the two forms the `SDI` admits.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        let ok = match value.len() {
            11 => value.bytes().all(|b| b.is_ascii_digit()) && luhn_ok(value.as_bytes()),
            16 => {
                cf_structure_ok(value.as_bytes())
                    && cf_check_char(&value[..15]) == value.as_bytes()[15]
            }
            _ => false,
        };
        if ok {
            Ok(Self(value))
        } else {
            Err(IdError::CodiceFiscale(value))
        }
    }

    /// The value as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the value is the 16-character personal form.
    #[must_use]
    pub const fn is_personal(&self) -> bool {
        self.0.len() == 16
    }
}

id_newtype!(CodiceFiscale, field);

/// The `CAP` of a seat, five digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cap([u8; 5]);

impl Cap {
    /// The generic `00000` the Agenzia prescribes for a foreign address.
    pub const FOREIGN: Self = Self(*b"00000");

    /// Builds the `Cap` when `value` is five digits.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Cap`] when the value is not five digits.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if value.len() == 5 && value.bytes().all(|b| b.is_ascii_digit()) {
            let mut digits = [0_u8; 5];
            digits.copy_from_slice(value.as_bytes());
            Ok(Self(digits))
        } else {
            Err(IdError::Cap(value))
        }
    }

    /// The five digits.
    ///
    /// # Panics
    ///
    /// Panics when the five digits are not valid UTF-8, which the
    /// construction rules prevent.
    #[must_use]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_else(|_| panic!("five ASCII digits are valid UTF-8"))
    }
}

impl fmt::Display for Cap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Cap {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

impl TryFrom<&str> for Cap {
    type Error = IdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl TryFrom<String> for Cap {
    type Error = IdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

/// Declares the `Provincia` enum and its `ALL`, `code`, `Display`,
/// `FromStr` and `TryFrom` impls from a list of `name code city` rows.
macro_rules! provincia {
    ($($name:ident $code:literal $city:literal),* $(,)?) => {
        /// The `Provincia` of an Italian seat, the code of the current
        /// provinces and metropolitan cities, from the Istat census of
        /// administrative units of 21/02/2026.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Provincia {
            $( #[doc = concat!("The province of ", $city, ", code `", $code, "`.")]
            $name, )*
        }

        impl Provincia {
            // the repeat cannot count literals alone, so the row variant carries the count
            const COUNT: usize = [$( (1, Self::$name), )*].len();

            /// Every variant, one per code of the census.
            pub const ALL: [Self; Self::COUNT] = [$( Self::$name, )*];

            /// The two-letter `Provincia` code.
            #[must_use]
            pub const fn code(self) -> &'static str {
                match self {
                    $( Self::$name => $code, )*
                }
            }
        }

        impl fmt::Display for Provincia {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.code())
            }
        }

        impl FromStr for Provincia {
            type Err = IdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::ALL
                    .iter()
                    .find(|provincia| provincia.code() == s)
                    .copied()
                    .ok_or_else(|| IdError::Provincia(s.to_string()))
            }
        }

        impl TryFrom<&str> for Provincia {
            type Error = IdError;

            fn try_from(s: &str) -> Result<Self, Self::Error> {
                Self::from_str(s)
            }
        }

        impl TryFrom<String> for Provincia {
            type Error = IdError;

            fn try_from(s: String) -> Result<Self, Self::Error> {
                Self::from_str(&s)
            }
        }
    };
}

provincia! {
    To "TO" "Torino",
    Vc "VC" "Vercelli",
    No "NO" "Novara",
    Cn "CN" "Cuneo",
    At "AT" "Asti",
    Al "AL" "Alessandria",
    Ao "AO" "Valle d'Aosta",
    Im "IM" "Imperia",
    Sv "SV" "Savona",
    Ge "GE" "Genova",
    Sp "SP" "La Spezia",
    Va "VA" "Varese",
    Co "CO" "Como",
    So "SO" "Sondrio",
    Mi "MI" "Milano",
    Bg "BG" "Bergamo",
    Bs "BS" "Brescia",
    Pv "PV" "Pavia",
    Cr "CR" "Cremona",
    Mn "MN" "Mantova",
    Bz "BZ" "Bolzano",
    Tn "TN" "Trento",
    Vr "VR" "Verona",
    Vi "VI" "Vicenza",
    Bl "BL" "Belluno",
    Tv "TV" "Treviso",
    Ve "VE" "Venezia",
    Pd "PD" "Padova",
    Ro "RO" "Rovigo",
    Ud "UD" "Udine",
    Go "GO" "Gorizia",
    Ts "TS" "Trieste",
    Pc "PC" "Piacenza",
    Pr "PR" "Parma",
    Re "RE" "Reggio Emilia",
    Mo "MO" "Modena",
    Bo "BO" "Bologna",
    Fe "FE" "Ferrara",
    Ra "RA" "Ravenna",
    Fc "FC" "Forlì-Cesena",
    Pu "PU" "Pesaro e Urbino",
    An "AN" "Ancona",
    Mc "MC" "Macerata",
    Ap "AP" "Ascoli Piceno",
    Ms "MS" "Massa-Carrara",
    Lu "LU" "Lucca",
    Pt "PT" "Pistoia",
    Fi "FI" "Firenze",
    Li "LI" "Livorno",
    Po "PO" "Prato",
    Pi "PI" "Pisa",
    Ar "AR" "Arezzo",
    Si "SI" "Siena",
    Gr "GR" "Grosseto",
    Pg "PG" "Perugia",
    Tr "TR" "Terni",
    Vt "VT" "Viterbo",
    Ri "RI" "Rieti",
    Rm "RM" "Roma",
    Lt "LT" "Latina",
    Fr "FR" "Frosinone",
    Ce "CE" "Caserta",
    Bn "BN" "Benevento",
    Na "NA" "Napoli",
    Av "AV" "Avellino",
    Sa "SA" "Salerno",
    Aq "AQ" "L'Aquila",
    Te "TE" "Teramo",
    Pe "PE" "Pescara",
    Ch "CH" "Chieti",
    Cb "CB" "Campobasso",
    Fg "FG" "Foggia",
    Ba "BA" "Bari",
    Ta "TA" "Taranto",
    Br "BR" "Brindisi",
    Le "LE" "Lecce",
    Pz "PZ" "Potenza",
    Mt "MT" "Matera",
    Cs "CS" "Cosenza",
    Cz "CZ" "Catanzaro",
    Rc "RC" "Reggio Calabria",
    Tp "TP" "Trapani",
    Pa "PA" "Palermo",
    Me "ME" "Messina",
    Ag "AG" "Agrigento",
    Cl "CL" "Caltanissetta",
    En "EN" "Enna",
    Ct "CT" "Catania",
    Rg "RG" "Ragusa",
    Sr "SR" "Siracusa",
    Pn "PN" "Pordenone",
    Is "IS" "Isernia",
    Bi "BI" "Biella",
    Lc "LC" "Lecco",
    Lo "LO" "Lodi",
    Rn "RN" "Rimini",
    Kr "KR" "Crotone",
    Vv "VV" "Vibo Valentia",
    Vb "VB" "Verbano-Cusio-Ossola",
    Mb "MB" "Monza e della Brianza",
    Fm "FM" "Fermo",
    Bt "BT" "Barletta-Andria-Trani",
    Ss "SS" "Sassari",
    Ot "OT" "Gallura Nord-Est Sardegna",
    Nu "NU" "Nuoro",
    Or "OR" "Oristano",
    Og "OG" "Ogliastra",
    Vs "VS" "Medio Campidano",
    Ca "CA" "Cagliari",
    Ci "CI" "Sulcis Iglesiente",
}

/// The `CodiceDestinatario` that identifies the buyer at the `IPA` and the
/// `SDI`, six characters for a public administration and seven for a
/// private party.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CodiceDestinatario(String);

impl CodiceDestinatario {
    /// The code the `SDI` assigns to a non-resident buyer.
    pub const FOREIGN: &str = "XXXXXXX";
    /// The code of the `PEC` delivery or of a buyer without a code.
    pub const PEC: &str = "0000000";

    /// Builds the `CodiceDestinatario` when `value` is six or seven
    /// uppercase alphanumeric characters.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::CodiceDestinatario`] when the value is not six
    /// or seven uppercase alphanumeric characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if (6..=7).contains(&value.len())
            && value
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        {
            Ok(Self(value))
        } else {
            Err(IdError::CodiceDestinatario(value))
        }
    }

    /// The code as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the code is the six-character form of a public
    /// administration, which carries the document in the `FPA12` format.
    #[must_use]
    pub const fn is_public_administration(&self) -> bool {
        self.0.len() == 6
    }
}

id_newtype!(CodiceDestinatario, field);

/// The `ProgressivoInvio` of the transmission, one to five alphanumeric
/// characters, shared with the file progressive of `InvoiceName`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProgressivoInvio(String);

impl ProgressivoInvio {
    /// Builds the `ProgressivoInvio` when `value` is one to five
    /// alphanumeric characters.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::ProgressivoInvio`] when the value is not one
    /// to five alphanumeric characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        crate::filename::check_progressive(&value)
            .map(Self)
            .map_err(|_| IdError::ProgressivoInvio(value))
    }

    /// The progressive as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The progressive as a `String`, consuming the value.
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

id_newtype!(ProgressivoInvio, field);

/// The `CodiceDestinatario` of a seat with a `BIC`, the eight or eleven
/// character code of the XSD.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Bic(String);

impl Bic {
    /// Builds the `Bic` when `value` fits the XSD pattern, six letters, a
    /// letter or the digits `2` to `9`, a letter but `O` or a digit, and an
    /// optional triplet of letters and digits.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Bic`] when the value does not fit the XSD pattern.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        let bytes = value.as_bytes();
        let ok = match bytes.len() {
            8 | 11 => {
                bytes[0..6].iter().all(u8::is_ascii_uppercase)
                    && matches!(bytes[6], b'A'..=b'Z' | b'2'..=b'9')
                    && matches!(bytes[7], b'A'..=b'N' | b'P'..=b'Z' | b'0'..=b'9')
                    && (bytes.len() == 8
                        || bytes[8..11]
                            .iter()
                            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()))
            }
            _ => false,
        };
        if ok {
            Ok(Self(value))
        } else {
            Err(IdError::Bic(value))
        }
    }

    /// The code as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(Bic, field);

/// The `IdCodice` of a non-Italian party, one to 28 uppercase alphanumeric
/// characters, as the Agenzia FAQ allows with no validity check by the
/// `SDI`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ForeignTaxId(String);

impl ForeignTaxId {
    /// Builds the `ForeignTaxId` when `value` is one to 28 uppercase
    /// alphanumeric characters.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::ForeignTaxId`] when the value is not one to 28
    /// uppercase alphanumeric characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if (1..=28).contains(&value.len())
            && value
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        {
            Ok(Self(value))
        } else {
            Err(IdError::ForeignTaxId(value))
        }
    }

    /// The identifier as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(ForeignTaxId, field);

/// The `Telefono` of the party contacts, five to 12 printable ASCII
/// characters per `TelFaxType`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Telefono(String);

impl Telefono {
    /// Builds the `Telefono` when `value` is five to 12 printable ASCII
    /// characters.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Telefono`] when the value is not five to 12
    /// printable ASCII characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if (5..=12).contains(&value.len()) && value.bytes().all(|b| (0x20_u8..=0x7E).contains(&b)) {
            Ok(Self(value))
        } else {
            Err(IdError::Telefono(value))
        }
    }

    /// The number as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(Telefono, field);

/// The `PECDestinatario` of the transmission, an e-mail of at most 256
/// characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pec(email_address::EmailAddress);

impl Pec {
    /// Builds the `Pec` when `value` is an e-mail of at most 256
    /// characters.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Pec`] when the value is not an e-mail of at
    /// most 256 characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if value.chars().count() <= 256 {
            email_address::EmailAddress::from_str(value.as_str())
                .map(Self)
                .map_err(|_| IdError::Pec(value))
        } else {
            Err(IdError::Pec(value))
        }
    }

    /// The address as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

id_newtype!(Pec, accessor);

/// The `Email` of the party contacts, an e-mail of seven to 256 characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Email(email_address::EmailAddress);

impl Email {
    /// Builds the `Email` when `value` is an e-mail of seven to 256
    /// characters.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Email`] when the value is not an e-mail of
    /// seven to 256 characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if (7..=256).contains(&value.chars().count()) {
            email_address::EmailAddress::from_str(value.as_str())
                .map(Self)
                .map_err(|_| IdError::Email(value))
        } else {
            Err(IdError::Email(value))
        }
    }

    /// The address as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

id_newtype!(Email, accessor);

/// The `IBAN` of the payment details, with the mod-97 check digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Iban(iban::Iban);

impl Iban {
    /// Builds the `Iban` when `value` passes the mod-97 check. Spaces are
    /// admitted on input and dropped, since the XSD forbids them.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Iban`] when the value fails the mod-97 check.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        iban::Iban::from_str(&value)
            .map(Self)
            .map_err(|_| IdError::Iban(value))
    }

    /// The code without spaces, as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.electronic_str()
    }
}

id_newtype!(Iban, accessor);

/// The `AliquotaIVA` of a taxable line or riepilogo, a percentage above
/// zero and at most 100 with two decimals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Aliquota(Decimal);

impl Aliquota {
    /// Builds the `Aliquota` when `value` is above zero, at most 100 and
    /// carries at most two decimals.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Aliquota`] when the value is not above zero and
    /// at most 100 with two decimals.
    pub fn try_new(value: impl Into<Decimal>) -> Result<Self, IdError> {
        let mut value = value.into();
        let original = value;
        value.rescale(2);
        if value == original && Decimal::ZERO < value && value <= Decimal::ONE_HUNDRED {
            Ok(Self(value))
        } else {
            Err(IdError::Aliquota(original.to_string()))
        }
    }

    /// The rate as a percentage with two decimals.
    #[must_use]
    pub const fn as_decimal(&self) -> Decimal {
        self.0
    }
}

impl fmt::Display for Aliquota {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Aliquota {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Decimal>()
            .map_err(|_| IdError::Aliquota(s.to_string()))
            .and_then(Self::try_new)
    }
}

impl TryFrom<&str> for Aliquota {
    type Error = IdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl TryFrom<String> for Aliquota {
    type Error = IdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl TryFrom<Decimal> for Aliquota {
    type Error = IdError;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

/// The `RiferimentoNormativo` of the riepilogo, one to 100 characters of
/// the XSD `String100Latin` type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RiferimentoNormativo(String);

impl RiferimentoNormativo {
    /// Builds the `RiferimentoNormativo` when `value` is one to 100
    /// characters of basic Latin and the Latin-1 supplement.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::RiferimentoNormativo`] when the value is not
    /// one to 100 Latin characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if latin1_len(&value, 100) {
            Ok(Self(value))
        } else {
            Err(IdError::RiferimentoNormativo(value))
        }
    }

    /// The reference as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(RiferimentoNormativo, field);

/// The exchange rate of a foreign currency document, the units of the
/// foreign currency per euro, of the art. 13 convention.
///
/// The rate carries at most four decimals, the precision of the Banca
/// d'Italia rates published under the art. 13 convention, and is above
/// zero, since a zero or negative rate cannot convert an amount. The
/// value is written with four decimals, that precision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExchangeRate(Decimal);

impl ExchangeRate {
    /// Builds the `ExchangeRate` when `value` is above zero and carries at
    /// most four decimals.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the value is not above zero or carries more than
    /// four decimals.
    pub fn try_new(value: impl Into<Decimal>) -> Result<Self, IdError> {
        let original = value.into();
        let mut value = original;
        value.rescale(4);
        if value == original && Decimal::ZERO < value {
            Ok(Self(value))
        } else {
            Err(IdError::ExchangeRate(original.to_string()))
        }
    }

    /// The units of the foreign currency per euro.
    #[must_use]
    pub const fn as_decimal(&self) -> Decimal {
        self.0
    }
}

impl fmt::Display for ExchangeRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for ExchangeRate {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Decimal>()
            .map_err(|_| IdError::ExchangeRate(s.to_string()))
            .and_then(Self::try_new)
    }
}

impl TryFrom<&str> for ExchangeRate {
    type Error = IdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl TryFrom<String> for ExchangeRate {
    type Error = IdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl TryFrom<Decimal> for ExchangeRate {
    type Error = IdError;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

/// The `Numero` of the `DatiGeneraliDocumento`, one to 20 printable basic
/// Latin characters, with at least one digit per the 00425 check of the
/// Elenco Controlli.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Numero(String);

impl Numero {
    /// Builds the `Numero` when `value` is one to 20 printable basic Latin
    /// characters and carries at least one digit.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Numero`] when the value is not one to 20
    /// printable basic Latin characters with at least one digit.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if basic_latin_len(&value, 20) && value.bytes().any(|b| b.is_ascii_digit()) {
            Ok(Self(value))
        } else {
            Err(IdError::Numero(value))
        }
    }

    /// The number as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(Numero, field);

/// The `IdDocumento` of the `DatiFattureCollegate`, one to 20 printable
/// basic Latin characters.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IdDocumento(String);

impl IdDocumento {
    /// Builds the `IdDocumento` when `value` is one to 20 printable basic
    /// Latin characters.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::IdDocumento`] when the value is not one to 20
    /// printable basic Latin characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if basic_latin_len(&value, 20) {
            Ok(Self(value))
        } else {
            Err(IdError::IdDocumento(value))
        }
    }

    /// The identifier as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(IdDocumento, field);

/// The `Causale` of the `DatiGeneraliDocumento`, one to 200 characters of
/// basic Latin and the Latin-1 supplement.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Causale(String);

impl Causale {
    /// Builds the `Causale` when `value` is one to 200 characters of basic
    /// Latin and the Latin-1 supplement.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Causale`] when the value is not one to 200
    /// Latin characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if latin1_len(&value, 200) {
            Ok(Self(value))
        } else {
            Err(IdError::Causale(value))
        }
    }

    /// The causale as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(Causale, field);

/// The `Descrizione` of the `DettaglioLinee`, one to 1000 characters of
/// basic Latin and the Latin-1 supplement.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Description(String);

impl Description {
    /// Builds the `Description` when `value` is one to 1000 characters of
    /// basic Latin and the Latin-1 supplement.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Description`] when the value is not one to
    /// 1000 Latin characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if latin1_len(&value, 1000) {
            Ok(Self(value))
        } else {
            Err(IdError::Description(value))
        }
    }

    /// The description as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(Description, field);

/// The `Denominazione` of the `Anagrafica` of a business, one to 80
/// characters of basic Latin and the Latin-1 supplement.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Denomination(String);

impl Denomination {
    /// Builds the `Denomination` when `value` is one to 80 characters of
    /// basic Latin and the Latin-1 supplement.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Denomination`] when the value is not one to
    /// 80 Latin characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if latin1_len(&value, 80) {
            Ok(Self(value))
        } else {
            Err(IdError::Denomination(value))
        }
    }

    /// The denomination as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(Denomination, field);

/// The `Nome` and the `Cognome` of the `Anagrafica` of a person, one to 60
/// characters of basic Latin and the Latin-1 supplement.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PersonName(String);

impl PersonName {
    /// Builds the `PersonName` when `value` is one to 60 characters of
    /// basic Latin and the Latin-1 supplement.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::PersonName`] when the value is not one to 60
    /// Latin characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if latin1_len(&value, 60) {
            Ok(Self(value))
        } else {
            Err(IdError::PersonName(value))
        }
    }

    /// The name as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(PersonName, field);

/// The `Indirizzo` of a seat, one to 60 characters of basic Latin and the
/// Latin-1 supplement, before the foreign postal code is appended.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Indirizzo(String);

impl Indirizzo {
    /// Builds the `Indirizzo` when `value` is one to 60 characters of basic
    /// Latin and the Latin-1 supplement.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Indirizzo`] when the value is not one to 60
    /// Latin characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if latin1_len(&value, 60) {
            Ok(Self(value))
        } else {
            Err(IdError::Indirizzo(value))
        }
    }

    /// The address as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(Indirizzo, field);

/// The `Comune` of a seat, one to 60 characters of basic Latin and the
/// Latin-1 supplement.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Comune(String);

impl Comune {
    /// Builds the `Comune` when `value` is one to 60 characters of basic
    /// Latin and the Latin-1 supplement.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::Comune`] when the value is not one to 60
    /// Latin characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if latin1_len(&value, 60) {
            Ok(Self(value))
        } else {
            Err(IdError::Comune(value))
        }
    }

    /// The comune as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(Comune, field);

/// The `UnitaMisura` of a line, one to 10 printable basic Latin
/// characters.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnitaMisura(String);

impl UnitaMisura {
    /// Builds the `UnitaMisura` when `value` is one to 10 printable basic
    /// Latin characters.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::UnitaMisura`] when the value is not one to 10
    /// printable basic Latin characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if basic_latin_len(&value, 10) {
            Ok(Self(value))
        } else {
            Err(IdError::UnitaMisura(value))
        }
    }

    /// The unit of measure as written in the document.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(UnitaMisura, field);

/// The foreign postal code appended to the `Indirizzo` of a foreign seat,
/// one to 10 uppercase alphanumeric characters, spaces or hyphens.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PostalCode(String);

impl PostalCode {
    /// Builds the `PostalCode` when `value` is one to 10 uppercase
    /// alphanumeric characters, spaces or hyphens.
    ///
    /// # Errors
    ///
    /// Fails with [`IdError::PostalCode`] when the value is not one to 10
    /// uppercase alphanumeric characters, spaces or hyphens.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        let ok = (1..=10).contains(&value.chars().count())
            && value
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b' ' || b == b'-')
            && value
                .bytes()
                .any(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
        if ok {
            Ok(Self(value))
        } else {
            Err(IdError::PostalCode(value))
        }
    }

    /// The postal code as appended to the address.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

id_newtype!(PostalCode, field);

/// One of the 27 EU member states, by its `ISO 3166-1` alpha-2 code, for
/// the elements where the EU and non-EU rules differ, the art. 7-ter
/// reverse charge and the `TD17` versus `TD19` self-invoice.
///
/// Greece carries the ISO code `GR`. The EU VAT identifier `EL` of the
/// VAT directive does not appear in a `FatturaPA` document, whose
/// `IdPaese` and `Nazione` are `ISO 3166-1` codes, so the variant is
/// `EuCountry::Gr`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EuCountry {
    /// `AT`, Austria.
    At,
    /// `BE`, Belgium.
    Be,
    /// `BG`, Bulgaria.
    Bg,
    /// `CY`, Cyprus.
    Cy,
    /// `CZ`, Czechia.
    Cz,
    /// `DE`, Germany.
    De,
    /// `DK`, Denmark.
    Dk,
    /// `EE`, Estonia.
    Ee,
    /// `ES`, Spain.
    Es,
    /// `FI`, Finland.
    Fi,
    /// `FR`, France.
    Fr,
    /// `GR`, Greece.
    Gr,
    /// `HR`, Croatia.
    Hr,
    /// `HU`, Hungary.
    Hu,
    /// `IE`, Ireland.
    Ie,
    /// `IT`, Italy.
    It,
    /// `LT`, Lithuania.
    Lt,
    /// `LU`, Luxembourg.
    Lu,
    /// `LV`, Latvia.
    Lv,
    /// `MT`, Malta.
    Mt,
    /// `NL`, the Netherlands.
    Nl,
    /// `PL`, Poland.
    Pl,
    /// `PT`, Portugal.
    Pt,
    /// `RO`, Romania.
    Ro,
    /// `SE`, Sweden.
    Se,
    /// `SI`, Slovenia.
    Si,
    /// `SK`, Slovakia.
    Sk,
}

impl EuCountry {
    /// The 27 member states, one per variant.
    pub const ALL: [Self; 27] = [
        Self::At,
        Self::Be,
        Self::Bg,
        Self::Cy,
        Self::Cz,
        Self::De,
        Self::Dk,
        Self::Ee,
        Self::Es,
        Self::Fi,
        Self::Fr,
        Self::Gr,
        Self::Hr,
        Self::Hu,
        Self::Ie,
        Self::It,
        Self::Lt,
        Self::Lu,
        Self::Lv,
        Self::Mt,
        Self::Nl,
        Self::Pl,
        Self::Pt,
        Self::Ro,
        Self::Se,
        Self::Si,
        Self::Sk,
    ];

    /// The `ISO 3166-1` alpha-2 code as written in the document.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::At => "AT",
            Self::Be => "BE",
            Self::Bg => "BG",
            Self::Cy => "CY",
            Self::Cz => "CZ",
            Self::De => "DE",
            Self::Dk => "DK",
            Self::Ee => "EE",
            Self::Es => "ES",
            Self::Fi => "FI",
            Self::Fr => "FR",
            Self::Gr => "GR",
            Self::Hr => "HR",
            Self::Hu => "HU",
            Self::Ie => "IE",
            Self::It => "IT",
            Self::Lt => "LT",
            Self::Lu => "LU",
            Self::Lv => "LV",
            Self::Mt => "MT",
            Self::Nl => "NL",
            Self::Pl => "PL",
            Self::Pt => "PT",
            Self::Ro => "RO",
            Self::Se => "SE",
            Self::Si => "SI",
            Self::Sk => "SK",
        }
    }
}

impl fmt::Display for EuCountry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for EuCountry {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .find(|country| country.code() == s)
            .copied()
            .ok_or_else(|| IdError::EuCountry(s.to_string()))
    }
}

impl TryFrom<&str> for EuCountry {
    type Error = IdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl TryFrom<String> for EuCountry {
    type Error = IdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

/// The `Natura` of a non-taxable line, per the codes of the specification.
/// The generic values the `SDI` retired (`N2`, `N3`, `N6`, check 00445)
/// are not constructible.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Natura {
    /// `N1`, the operation excluded from VAT, ex art. 15 DPR 633/72.
    N1,
    /// `N2.1`, an operation not subject to VAT, art. 7-ter DPR 633/72.
    N2_1,
    /// `N2.2`, an operation not subject to VAT, art. 7-bis DPR 633/72.
    N2_2,
    /// `N3.1`, a non-taxable operation, art. 9-13 DPR 633/72.
    N3_1,
    /// `N3.2`, an export of goods or service, art. 9-13 DPR 633/72.
    N3_2,
    /// `N3.3`, a sale to passengers in transit.
    N3_3,
    /// `N3.4`, an operation exempt from VAT.
    N3_4,
    /// `N3.5`, a non-taxable intra-community sale.
    N3_5,
    /// `N3.6`, an operation with a specific exemption.
    N3_6,
    /// `N4`, an operation outside the scope of VAT.
    N4,
    /// `N5`, an operation with the right to deduction.
    N5,
    /// `N6.1`, the VAT due at receipt or payment, goods.
    N6_1,
    /// `N6.2`, the VAT due at receipt or payment, goods with the right to
    /// deduction.
    N6_2,
    /// `N6.3`, the VAT due at receipt or payment, services.
    N6_3,
    /// `N6.4`, the VAT due at receipt or payment, services with the right
    /// to deduction.
    N6_4,
    /// `N6.5`, the VAT due at receipt or payment, an intra-community sale.
    N6_5,
    /// `N6.6`, the VAT due at receipt or payment, an import of goods.
    N6_6,
    /// `N6.7`, the VAT due at receipt or payment, a specific regime.
    N6_7,
    /// `N6.8`, the VAT due at receipt or payment, a specific regime for
    /// services.
    N6_8,
    /// `N6.9`, an other operation with the VAT due at receipt or payment.
    N6_9,
    /// `N7`, the VAT due at receipt or payment, goods.
    N7,
}

impl Natura {
    /// The current codes of the schema, the retired generic `N2`, `N3`
    /// and `N6` aside, one per variant.
    pub const ALL: [Self; 21] = [
        Self::N1,
        Self::N2_1,
        Self::N2_2,
        Self::N3_1,
        Self::N3_2,
        Self::N3_3,
        Self::N3_4,
        Self::N3_5,
        Self::N3_6,
        Self::N4,
        Self::N5,
        Self::N6_1,
        Self::N6_2,
        Self::N6_3,
        Self::N6_4,
        Self::N6_5,
        Self::N6_6,
        Self::N6_7,
        Self::N6_8,
        Self::N6_9,
        Self::N7,
    ];

    /// The code as written in the `Natura` element, such as `N2.1`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::N1 => "N1",
            Self::N2_1 => "N2.1",
            Self::N2_2 => "N2.2",
            Self::N3_1 => "N3.1",
            Self::N3_2 => "N3.2",
            Self::N3_3 => "N3.3",
            Self::N3_4 => "N3.4",
            Self::N3_5 => "N3.5",
            Self::N3_6 => "N3.6",
            Self::N4 => "N4",
            Self::N5 => "N5",
            Self::N6_1 => "N6.1",
            Self::N6_2 => "N6.2",
            Self::N6_3 => "N6.3",
            Self::N6_4 => "N6.4",
            Self::N6_5 => "N6.5",
            Self::N6_6 => "N6.6",
            Self::N6_7 => "N6.7",
            Self::N6_8 => "N6.8",
            Self::N6_9 => "N6.9",
            Self::N7 => "N7",
        }
    }

    /// The generated `NaturaType` value of the code.
    #[must_use]
    pub const fn generated(self) -> NaturaType {
        match self {
            Self::N1 => NaturaType::N1,
            Self::N2_1 => NaturaType::N21,
            Self::N2_2 => NaturaType::N22,
            Self::N3_1 => NaturaType::N31,
            Self::N3_2 => NaturaType::N32,
            Self::N3_3 => NaturaType::N33,
            Self::N3_4 => NaturaType::N34,
            Self::N3_5 => NaturaType::N35,
            Self::N3_6 => NaturaType::N36,
            Self::N4 => NaturaType::N4,
            Self::N5 => NaturaType::N5,
            Self::N6_1 => NaturaType::N61,
            Self::N6_2 => NaturaType::N62,
            Self::N6_3 => NaturaType::N63,
            Self::N6_4 => NaturaType::N64,
            Self::N6_5 => NaturaType::N65,
            Self::N6_6 => NaturaType::N66,
            Self::N6_7 => NaturaType::N67,
            Self::N6_8 => NaturaType::N68,
            Self::N6_9 => NaturaType::N69,
            Self::N7 => NaturaType::N7,
        }
    }
}

impl fmt::Display for Natura {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// The `RegimeFiscale` of the `CedentePrestatore`, one of the codes of the
/// XSD enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RegimeFiscale {
    /// `RF01`, the ordinary regime.
    Rf01,
    /// `RF02`, the regime of the minimal taxpayers, art. 1 c. 96-117 L.
    /// 244/2007.
    Rf02,
    /// `RF04`, agriculture and related activities and fishing, art. 34 and
    /// 34-bis DPR 633/1972.
    Rf04,
    /// `RF05`, the sale of salt and tobacco, art. 74 c. 1 DPR 633/1972.
    Rf05,
    /// `RF06`, the trade of matches, art. 74 c. 1 DPR 633/1972.
    Rf06,
    /// `RF07`, publishing, art. 74 c. 1 DPR 633/1972.
    Rf07,
    /// `RF08`, the management of public telephone services, art. 74 c. 1
    /// DPR 633/1972.
    Rf08,
    /// `RF09`, the resale of public transport and parking documents,
    /// art. 74 c. 1 DPR 633/1972.
    Rf09,
    /// `RF10`, entertainment, games and the activities of the tariff of
    /// DPR 640/1972, art. 74 c. 6 DPR 633/1972.
    Rf10,
    /// `RF11`, travel and tourism agencies, art. 74-ter DPR 633/1972.
    Rf11,
    /// `RF12`, agriturismo, art. 5 c. 2 L. 413/1991.
    Rf12,
    /// `RF13`, sales at home, art. 25-bis c. 6 DPR 600/1973.
    Rf13,
    /// `RF14`, the resale of used goods and of art, antique and collection
    /// objects, art. 36 D.L. 41/1995.
    Rf14,
    /// `RF15`, the auction agencies of art, antique and collection
    /// objects, art. 40-bis D.L. 41/1995.
    Rf15,
    /// `RF16`, the cash-basis VAT for the public administration, art. 6
    /// c. 5 DPR 633/1972.
    Rf16,
    /// `RF17`, the cash-basis VAT, art. 32-bis D.L. 83/2012.
    Rf17,
    /// `RF18`, other.
    Rf18,
    /// `RF19`, the forfettario regime.
    Rf19,
    /// `RF20`, the cross-border VAT franchise regime, EU directive
    /// 2020/285.
    Rf20,
}

impl RegimeFiscale {
    /// The codes of the XSD enumeration, one per variant.
    pub const ALL: [Self; 19] = [
        Self::Rf01,
        Self::Rf02,
        Self::Rf04,
        Self::Rf05,
        Self::Rf06,
        Self::Rf07,
        Self::Rf08,
        Self::Rf09,
        Self::Rf10,
        Self::Rf11,
        Self::Rf12,
        Self::Rf13,
        Self::Rf14,
        Self::Rf15,
        Self::Rf16,
        Self::Rf17,
        Self::Rf18,
        Self::Rf19,
        Self::Rf20,
    ];

    /// The code as written in the `RegimeFiscale` element.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Rf01 => "RF01",
            Self::Rf02 => "RF02",
            Self::Rf04 => "RF04",
            Self::Rf05 => "RF05",
            Self::Rf06 => "RF06",
            Self::Rf07 => "RF07",
            Self::Rf08 => "RF08",
            Self::Rf09 => "RF09",
            Self::Rf10 => "RF10",
            Self::Rf11 => "RF11",
            Self::Rf12 => "RF12",
            Self::Rf13 => "RF13",
            Self::Rf14 => "RF14",
            Self::Rf15 => "RF15",
            Self::Rf16 => "RF16",
            Self::Rf17 => "RF17",
            Self::Rf18 => "RF18",
            Self::Rf19 => "RF19",
            Self::Rf20 => "RF20",
        }
    }

    /// The generated `RegimeFiscaleType` value of the code.
    #[must_use]
    pub const fn generated(self) -> RegimeFiscaleType {
        match self {
            Self::Rf01 => RegimeFiscaleType::Rf01,
            Self::Rf02 => RegimeFiscaleType::Rf02,
            Self::Rf04 => RegimeFiscaleType::Rf04,
            Self::Rf05 => RegimeFiscaleType::Rf05,
            Self::Rf06 => RegimeFiscaleType::Rf06,
            Self::Rf07 => RegimeFiscaleType::Rf07,
            Self::Rf08 => RegimeFiscaleType::Rf08,
            Self::Rf09 => RegimeFiscaleType::Rf09,
            Self::Rf10 => RegimeFiscaleType::Rf10,
            Self::Rf11 => RegimeFiscaleType::Rf11,
            Self::Rf12 => RegimeFiscaleType::Rf12,
            Self::Rf13 => RegimeFiscaleType::Rf13,
            Self::Rf14 => RegimeFiscaleType::Rf14,
            Self::Rf15 => RegimeFiscaleType::Rf15,
            Self::Rf16 => RegimeFiscaleType::Rf16,
            Self::Rf17 => RegimeFiscaleType::Rf17,
            Self::Rf18 => RegimeFiscaleType::Rf18,
            Self::Rf19 => RegimeFiscaleType::Rf19,
            Self::Rf20 => RegimeFiscaleType::Rf20,
        }
    }
}

impl fmt::Display for RegimeFiscale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for RegimeFiscale {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .find(|regime| regime.code() == s)
            .copied()
            .ok_or_else(|| IdError::RegimeFiscale(s.to_string()))
    }
}

impl TryFrom<&str> for RegimeFiscale {
    type Error = IdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl TryFrom<String> for RegimeFiscale {
    type Error = IdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl FromStr for Natura {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .find(|natura| natura.code() == s)
            .copied()
            .ok_or_else(|| IdError::Natura(s.to_string()))
    }
}

impl TryFrom<&str> for Natura {
    type Error = IdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl TryFrom<String> for Natura {
    type Error = IdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

/// The value of every digit doubled, the digits above four reduced by
/// nine, per the Luhn algorithm.
const LUHN_DOUBLED: [u32; 10] = [0, 2, 4, 6, 8, 1, 3, 5, 7, 9];

/// The Luhn validity of an all-digit string, with every second digit from
/// the right doubled.
fn luhn_ok(digits: &[u8]) -> bool {
    let mut sum = 0_u32;
    let mut double = false;
    for &byte in digits.iter().rev() {
        // the caller guarantees an ASCII digit, so the subtraction is exact
        let digit = byte - b'0';
        if double {
            sum += LUHN_DOUBLED[usize::from(digit)];
        } else {
            sum += u32::from(digit);
        }
        double = !double;
    }
    sum.is_multiple_of(10)
}

/// The structure of the 16-character personal form, a letter at every
/// position but the seven digit positions, where an omocodia letter may
/// replace the digit.
fn cf_structure_ok(bytes: &[u8]) -> bool {
    debug_assert_eq!(bytes.len(), 16, "cf_structure_ok takes the 16 characters");
    bytes.iter().enumerate().all(|(i, b)| {
        if OMOCODIA_POSITIONS.contains(&i) {
            b.is_ascii_digit() || OMOCODIA.contains(b)
        } else {
            b.is_ascii_uppercase()
        }
    })
}

/// The 16th character of the personal form, the letter of the remainder of
/// the value sum divided by 26, per D.M. 23/12/1976 n. 1381 art. 7. A
/// digit takes the value of the letter at the same index at an odd
/// position and its own value at an even position.
fn cf_check_char(first15: &str) -> u8 {
    debug_assert_eq!(
        first15.len(),
        15,
        "cf_check_char takes the first 15 characters"
    );
    let mut sum = 0_u32;
    for (position, char) in first15.chars().enumerate() {
        // the 0-based position of the 1-based odd position is even
        let value = if char.is_ascii_uppercase() {
            // the matched range bounds the index to 0 to 25
            let index = (u32::from(char) - u32::from('A')) as usize;
            debug_assert!(index < 26, "the index is bounded by the range match");
            if position.is_multiple_of(2) {
                ODD_LETTER[index]
            } else {
                EVEN_LETTER[index]
            }
        } else {
            // a digit is a letter at the same index of the table
            // and an even position adds the digit's own value
            let value = u32::from(char) - u32::from('0');
            debug_assert!(value <= 9, "a digit is 0 to 9");
            let index = value as usize; // 0 to 9 fits a usize on every target
            if position.is_multiple_of(2) {
                ODD_LETTER[index]
            } else {
                value
            }
        };
        sum += value;
    }
    // A to Z fits a byte by construction of the sum
    b'A' + (sum % 26) as u8
}

/// Whether every character of `s` is printable basic Latin or of the
/// Latin-1 supplement, the character set of the XSD `StringLatin` types.
fn latin1_chars(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_ascii_graphic() || c == ' ' || (0xA0_u32..=0xFF_u32).contains(&u32::from(c)))
}

/// Whether `s` is one to `max` characters of basic Latin and the Latin-1
/// supplement.
fn latin1_len(s: &str, max: usize) -> bool {
    !s.is_empty() && s.chars().count() <= max && latin1_chars(s)
}

/// Whether `s` is one to `max` printable basic Latin characters, the
/// character set of the XSD `String10Type` and `String20Type` types.
fn basic_latin_len(s: &str, max: usize) -> bool {
    !s.is_empty() && s.chars().count() <= max && s.bytes().all(|b| (0x20_u8..=0x7E_u8).contains(&b))
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing case"
)]
mod tests {
    use super::*;

    /// One row of the `latin1` text boundary table, the element name,
    /// the XSD boundary, the validating constructor and the error
    /// variant of the element.
    type LatinCase<'a> = (
        &'a str,
        usize,
        fn(&str) -> Result<(), IdError>,
        fn(String) -> IdError,
    );

    #[test]
    fn partita_iva_accepts_valid_and_rejects_invalid() {
        assert_eq!(
            PartitaIva::try_new("02222222222").expect("a valid partita IVA"),
            "02222222222"
                .parse::<PartitaIva>()
                .expect("a valid partita IVA")
        );
        assert_eq!(
            PartitaIva::try_new("04444444444"),
            Ok("04444444444".parse().expect("a valid partita IVA"))
        );
        for bad in [
            "01234567890",
            "00000000000",
            "1234567890",
            "0218203039A",
            " 02222222222",
            "022222222221",
        ] {
            assert!(
                matches!(PartitaIva::try_new(bad), Err(IdError::PartitaIva(_))),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn codice_fiscale_accepts_the_corpus_forms() {
        for good in [
            "MRALNE80E05H501C",
            "RSSGNN60R30H501U",
            "HVQWPH73P42H501Y",
            "BLDRFL86E05I452D",
            "CCCNNN80D07L407I",
            "RSSMRA80A01H501U",
        ] {
            let message = format!("{good} must be accepted");
            let cf = CodiceFiscale::try_new(good).expect(&message);
            assert!(cf.is_personal());
        }
        assert!(
            !CodiceFiscale::try_new("02222222222")
                .expect("a valid codice fiscale")
                .is_personal()
        );
        for bad in [
            // wrong check character
            "MRALNE80E05H501D",
            // a digit in a letter position
            "MRALNE80E05H5012",
            // a non-omocodia letter in a digit position
            "MRALNE80E0K5H501C",
            // a wrong length
            "MRALNE80E05H501",
            // eleven characters that pass the luhn sum but carry a letter
            "D2182030391",
            // a digit in a letter position whose check character still matches
            "MRA4NE80E05H501V",
        ] {
            assert!(
                matches!(CodiceFiscale::try_new(bad), Err(IdError::CodiceFiscale(_))),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn cap_round_trips_and_rejects() {
        assert_eq!(
            Cap::try_new("00100").expect("a valid CAP").to_string(),
            "00100"
        );
        assert_eq!(Cap::FOREIGN.to_string(), "00000");
        for bad in ["0010", "001001", "0010X", "abcd1"] {
            assert!(
                matches!(Cap::try_new(bad), Err(IdError::Cap(_))),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn provincia_round_trips_and_rejects() {
        assert_eq!(Provincia::To.code(), "TO");
        assert_eq!("TO".parse::<Provincia>(), Ok(Provincia::To));
        assert_eq!(Provincia::Ci.code(), "CI");
        assert!(matches!("CI".parse::<Provincia>(), Ok(Provincia::Ci)));
        for bad in ["to", "TOO", "XX", "12", "SU"] {
            assert!(
                matches!(bad.parse::<Provincia>(), Err(IdError::Provincia(_))),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn codice_destinatario_round_trips_and_rejects() {
        let codice =
            CodiceDestinatario::try_new(CodiceDestinatario::FOREIGN).expect("a valid code");
        assert_eq!(codice.to_string(), "XXXXXXX");
        assert!(!codice.is_public_administration());
        let pa = CodiceDestinatario::try_new("PA1234").expect("a valid code");
        assert!(pa.is_public_administration());
        for bad in ["ABC", "AB C12", "abcdef", "ABCDEFGH"] {
            assert!(
                matches!(
                    CodiceDestinatario::try_new(bad),
                    Err(IdError::CodiceDestinatario(_))
                ),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn progressivo_invio_round_trips_and_rejects() {
        assert_eq!(
            ProgressivoInvio::try_new("00001")
                .expect("a valid progressivo")
                .to_string(),
            "00001"
        );
        for bad in ["", "123456", "0000X1"] {
            assert!(
                matches!(
                    ProgressivoInvio::try_new(bad),
                    Err(IdError::ProgressivoInvio(_))
                ),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn bic_round_trips_and_rejects() {
        for good in ["DEUTDEFF501", "DEUTDEFF", "INGBNL2A", "AZADAD21XXX"] {
            assert!(Bic::try_new(good).is_ok(), "{good} must be accepted");
        }
        for bad in ["DEUTDEFF5011", "deutdeff501", "DEUTDE1F", "DEUTDEFF501111"] {
            assert!(
                matches!(Bic::try_new(bad), Err(IdError::Bic(_))),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn foreign_tax_id_round_trips_and_rejects() {
        assert_eq!(
            ForeignTaxId::try_new("DE998877665")
                .expect("a valid identifier")
                .to_string(),
            "DE998877665"
        );
        assert!(ForeignTaxId::try_new("X").is_ok());
        assert!(matches!(
            ForeignTaxId::try_new("A".repeat(29)),
            Err(IdError::ForeignTaxId(_))
        ));
        for bad in ["", "de998877665", "DE 99887766"] {
            assert!(
                matches!(ForeignTaxId::try_new(bad), Err(IdError::ForeignTaxId(_))),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn telefono_round_trips_and_rejects() {
        assert_eq!(
            Telefono::try_new("+390612345")
                .expect("a valid telephone number")
                .to_string(),
            "+390612345"
        );
        for bad in ["0612", "06123456789012", "06 123 45 67 89 0"] {
            assert!(
                matches!(Telefono::try_new(bad), Err(IdError::Telefono(_))),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn pec_and_email_round_trip_and_reject() {
        assert_eq!(
            Pec::try_new("pec@agenzia.it")
                .expect("a valid pec")
                .to_string(),
            "pec@agenzia.it"
        );
        assert!(matches!(Pec::try_new("not-an-email"), Err(IdError::Pec(_))));
        assert!(matches!(
            Pec::try_new(format!("{}@a.it", "a".repeat(260))),
            Err(IdError::Pec(_))
        ));
        assert!(Email::try_new("contact@example.com").is_ok());
        assert!(
            matches!(Email::try_new("a@b.c"), Err(IdError::Email(_))),
            "seven characters minimum"
        );
        assert!(matches!(
            Email::try_new(format!("{}@example.com", "a".repeat(250))),
            Err(IdError::Email(_))
        ));
    }

    #[test]
    fn iban_round_trips_and_rejects() {
        let iban = Iban::try_new("IT60X0542811101000000123456").expect("a valid IBAN");
        assert_eq!(iban.to_string(), "IT60X0542811101000000123456");
        assert_eq!(
            Iban::try_new("IT60 X054 2811 1010 0000 0123 456")
                .expect("a valid IBAN")
                .to_string(),
            "IT60X0542811101000000123456"
        );
        assert!(matches!(
            Iban::try_new("IT60X0542811101000000123457"),
            Err(IdError::Iban(_))
        ));
    }

    #[test]
    fn aliquota_round_trips_and_rejects() {
        assert_eq!(
            Aliquota::try_new(Decimal::ONE_HUNDRED)
                .expect("a valid rate")
                .to_string(),
            "100.00"
        );
        assert_eq!(
            Aliquota::try_new(Decimal::new(22, 0))
                .expect("a valid rate")
                .to_string(),
            "22.00"
        );
        assert_eq!(
            "22.00"
                .parse::<Aliquota>()
                .expect("a valid rate")
                .as_decimal(),
            Decimal::new(22, 0)
        );
        for bad in [
            Decimal::ZERO,
            -Decimal::ONE,
            Decimal::new(10001, 2),
            Decimal::new(1, 3),
        ] {
            assert!(
                matches!(Aliquota::try_new(bad), Err(IdError::Aliquota(_))),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn exchange_rate_round_trips_and_rejects() {
        assert_eq!(
            "1.08"
                .parse::<ExchangeRate>()
                .expect("a valid exchange rate")
                .to_string(),
            "1.0800"
        );
        assert_eq!(
            "1".parse::<ExchangeRate>()
                .expect("a valid exchange rate")
                .to_string(),
            "1.0000"
        );
        for text in ["1.1172", "96.1234", "0.1234"] {
            let rate = text.parse::<ExchangeRate>().expect("a valid exchange rate");
            assert_eq!(rate.to_string(), text, "the round trip of {text}");
        }
        assert_eq!(
            ExchangeRate::try_new(Decimal::new(108, 2))
                .expect("a valid exchange rate")
                .as_decimal(),
            Decimal::new(108, 2)
        );
        for bad in ["0", "0.0", "-1.08", "1.23456", "abc", ""] {
            assert!(
                matches!(bad.parse::<ExchangeRate>(), Err(IdError::ExchangeRate(_))),
                "{bad} must be rejected"
            );
        }
        assert!(matches!(
            ExchangeRate::try_new(-Decimal::ONE),
            Err(IdError::ExchangeRate(_))
        ));
        assert!(matches!(
            ExchangeRate::try_new(Decimal::new(123_456, 5)),
            Err(IdError::ExchangeRate(_))
        ));
    }

    /// The `Email` of seven to 256 characters that is not an e-mail is
    /// reported with the `Email` error, the length check aside.
    #[test]
    fn email_reports_the_address_error_when_in_range() {
        for bad in ["not an email", "a b@example.com", "no-at-sign.example"] {
            assert!(
                matches!(Email::try_new(bad), Err(IdError::Email(_))),
                "{bad:?} has the right length but is not an e-mail"
            );
        }
    }

    /// The `Aliquota` text that is not a decimal number is reported with
    /// the parsed text.
    #[test]
    fn aliquota_reports_the_parse_error_for_non_decimal_text() {
        for bad in ["abc", "", "22,00"] {
            let error = Aliquota::from_str(bad).expect_err("the text is rejected");
            assert!(
                matches!(error, IdError::Aliquota(text) if text == bad),
                "the error carries {bad:?}"
            );
            assert!(
                matches!(
                    Aliquota::try_from(String::from(bad)),
                    Err(IdError::Aliquota(text)) if text == bad
                ),
                "TryFrom<String> carries the same error"
            );
        }
        assert_eq!(
            Aliquota::try_from(String::from("22.00"))
                .expect("a valid rate")
                .to_string(),
            "22.00"
        );
    }

    /// The `Aliquota` conversion of a `Decimal` agrees with the
    /// constructor on the valid rates and reports the original text on
    /// the rejected ones.
    #[test]
    fn aliquota_from_decimal_converts_like_the_constructor() {
        for rate in [
            Decimal::new(22, 0),
            Decimal::ONE_HUNDRED,
            Decimal::new(5, 2),
            Decimal::new(1, 1),
        ] {
            assert_eq!(Aliquota::try_from(rate), Aliquota::try_new(rate));
        }
        assert_eq!(
            Aliquota::try_from(Decimal::new(22, 0))
                .expect("a valid rate")
                .to_string(),
            "22.00"
        );
        for bad in [
            Decimal::ZERO,
            -Decimal::ONE,
            Decimal::new(10001, 2),
            Decimal::new(1, 3),
        ] {
            assert_eq!(
                Aliquota::try_from(bad),
                Err(IdError::Aliquota(bad.to_string())),
                "{bad} is rejected with its original text"
            );
        }
    }

    /// The `ExchangeRate` conversion of a `Decimal` agrees with the
    /// constructor on the valid rates and reports the original text on
    /// the rejected ones.
    #[test]
    fn exchange_rate_from_decimal_converts_like_the_constructor() {
        for rate in [
            Decimal::new(10800, 4),
            Decimal::new(96_1234, 4),
            Decimal::new(1, 0),
        ] {
            assert_eq!(ExchangeRate::try_from(rate), ExchangeRate::try_new(rate));
        }
        assert_eq!(
            ExchangeRate::try_from(Decimal::new(10800, 4))
                .expect("a valid rate")
                .to_string(),
            "1.0800"
        );
        for bad in [Decimal::ZERO, -Decimal::ONE, Decimal::new(123_456, 5)] {
            assert_eq!(
                ExchangeRate::try_from(bad),
                Err(IdError::ExchangeRate(bad.to_string())),
                "{bad} is rejected with its original text"
            );
        }
    }

    #[test]
    fn riferimento_normativo_round_trips_and_rejects() {
        let ref_ =
            RiferimentoNormativo::try_new("ART. 7-TER DPR 633/72").expect("a valid reference");
        assert_eq!(ref_.to_string(), "ART. 7-TER DPR 633/72");
        assert!(RiferimentoNormativo::try_new("Forlì-Cesena é qui").is_ok());
        assert!(matches!(
            RiferimentoNormativo::try_new(""),
            Err(IdError::RiferimentoNormativo(_))
        ));
        assert!(matches!(
            RiferimentoNormativo::try_new("a".repeat(101)),
            Err(IdError::RiferimentoNormativo(_))
        ));
    }

    #[test]
    fn eu_country_uses_the_iso_codes() {
        assert_eq!(EuCountry::Gr.code(), "GR");
        assert_eq!("GR".parse::<EuCountry>(), Ok(EuCountry::Gr));
        assert!(matches!(
            "EL".parse::<EuCountry>(),
            Err(IdError::EuCountry(_))
        ));
        assert_eq!(EuCountry::It.to_string(), "IT");
    }

    #[test]
    fn natura_codes_and_generated_values() {
        assert_eq!(Natura::N2_1.code(), "N2.1");
        assert_eq!("N2.1".parse::<Natura>(), Ok(Natura::N2_1));
        assert_eq!(Natura::N2_1.generated(), NaturaType::N21);
        assert_eq!(Natura::N6_9.generated(), NaturaType::N69);
        for bad in ["N2", "N3", "N6", "N2.3", "n2.1", "N99"] {
            assert!(
                matches!(bad.parse::<Natura>(), Err(IdError::Natura(_))),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn numero_is_one_to_20_basic_latin_with_a_digit() {
        assert!(Numero::try_new("1").is_ok(), "one character");
        assert!(
            Numero::try_new("01234567890123456789").is_ok(),
            "20 characters"
        );
        for bad in [
            "",
            "ABCDEFGHIJKL",
            "NUMERO\n",
            "NUMÉRO",
            "012345678901234567890",
        ] {
            assert!(
                matches!(Numero::try_new(bad), Err(IdError::Numero(_))),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn id_documento_is_one_to_20_basic_latin() {
        assert!(IdDocumento::try_new("F").is_ok(), "one character");
        assert!(
            IdDocumento::try_new("01234567890123456789").is_ok(),
            "20 characters"
        );
        for bad in ["", "012345678901234567890", "DOC\n", "DOCÉ"] {
            assert!(
                matches!(IdDocumento::try_new(bad), Err(IdError::IdDocumento(_))),
                "{bad} must be rejected"
            );
        }
    }

    /// The six `latin1` text newtypes enforce the boundary and the
    /// character set of their element, each reporting its own variant.
    #[test]
    fn latin_text_newtypes_enforce_the_xsd_boundaries() {
        let cases: [LatinCase<'_>; 6] = [
            (
                "Causale",
                200,
                |s| Causale::try_new(s).map(drop),
                IdError::Causale,
            ),
            (
                "Descrizione",
                1000,
                |s| Description::try_new(s).map(drop),
                IdError::Description,
            ),
            (
                "Denominazione",
                80,
                |s| Denomination::try_new(s).map(drop),
                IdError::Denomination,
            ),
            (
                "Nome",
                60,
                |s| PersonName::try_new(s).map(drop),
                IdError::PersonName,
            ),
            (
                "Indirizzo",
                60,
                |s| Indirizzo::try_new(s).map(drop),
                IdError::Indirizzo,
            ),
            (
                "Comune",
                60,
                |s| Comune::try_new(s).map(drop),
                IdError::Comune,
            ),
        ];
        for (name, bound, try_new, invalid) in cases {
            let at_bound = "a".repeat(bound);
            let over_bound = "a".repeat(bound + 1);
            assert!(try_new("a").is_ok(), "{name}: one character");
            assert!(try_new(&at_bound).is_ok(), "{name}: {bound} characters");
            assert!(
                try_new("Test é à ü").is_ok(),
                "{name}: the Latin-1 supplement"
            );
            for bad in ["", &over_bound, "TEST\n", "TEST€"] {
                assert!(
                    matches!(
                        try_new(bad),
                        Err(error) if error == invalid(bad.to_string())
                    ),
                    "{name}: the {bad:?} value must be rejected"
                );
            }
        }
    }

    #[test]
    fn unita_misura_is_one_to_10_basic_latin() {
        assert!(UnitaMisura::try_new("PZ").is_ok(), "two characters");
        assert!(UnitaMisura::try_new("0123456789").is_ok(), "10 characters");
        for bad in ["", "01234567890", "KG\n", "KGÉ"] {
            assert!(
                matches!(UnitaMisura::try_new(bad), Err(IdError::UnitaMisura(_))),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn postal_code_is_one_to_10_uppercase_alphanumeric() {
        assert!(PostalCode::try_new("1").is_ok(), "one character");
        assert!(PostalCode::try_new("0123456789").is_ok(), "10 characters");
        assert!(PostalCode::try_new("S-09011").is_ok(), "a hyphen");
        assert!(PostalCode::try_new("09 011").is_ok(), "a space");
        for bad in ["", "01234567890", "s-09011", "----", " "] {
            assert!(
                matches!(PostalCode::try_new(bad), Err(IdError::PostalCode(_))),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn regime_fiscale_round_trips_and_rejects() {
        assert_eq!(RegimeFiscale::Rf01.code(), "RF01");
        assert_eq!("RF01".parse::<RegimeFiscale>(), Ok(RegimeFiscale::Rf01));
        assert_eq!(RegimeFiscale::Rf20.code(), "RF20");
        assert_eq!("RF20".parse::<RegimeFiscale>(), Ok(RegimeFiscale::Rf20));
        for bad in ["RF03", "rf01", "RF99"] {
            assert!(
                matches!(bad.parse::<RegimeFiscale>(), Err(IdError::RegimeFiscale(_))),
                "{bad} must be rejected"
            );
        }
    }

    /// The `Display` output of `T` re-parses through `FromStr` to the
    /// same value, the text accessor and `AsRef<str>` return the parsed
    /// text, and the `TryFrom<&str>` and `TryFrom<String>` conversions
    /// agree with `FromStr` on a valid and on an invalid value.
    fn round_trips<T, F>(valid: &str, invalid: &str, text: F)
    where
        T: Sized
            + fmt::Debug
            + fmt::Display
            + PartialEq
            + AsRef<str>
            + FromStr<Err = IdError>
            + for<'a> TryFrom<&'a str, Error = IdError>
            + TryFrom<String, Error = IdError>,
        F: Fn(&T) -> String,
    {
        let value = T::from_str(valid).expect("the valid value parses");
        assert_eq!(
            text(&value),
            valid,
            "the text accessor of the parsed {valid}"
        );
        assert_eq!(
            value.as_ref(),
            valid,
            "`AsRef<str>` reads the parsed {valid} back"
        );
        let rendered = value.to_string();
        assert_eq!(rendered, valid, "the display output of the parsed {valid}");
        assert_eq!(
            T::from_str(&rendered).expect("the display output parses"),
            value,
            "the display output re-parses to the same value"
        );
        assert_eq!(
            T::try_from(rendered.as_str()).expect("the display output converts"),
            value,
            "TryFrom agrees with FromStr on the valid value"
        );
        assert_eq!(
            T::try_from(valid.to_string()).expect("the owned text converts"),
            value,
            "TryFrom<String> agrees with FromStr on the valid value"
        );
        assert_eq!(
            T::from_str(invalid).map_err(|e| e.to_string()),
            T::try_from(invalid.to_string()).map_err(|e| e.to_string()),
            "TryFrom<String> reports the same error as FromStr"
        );
        assert_eq!(
            T::from_str(invalid).is_err(),
            T::try_from(invalid).is_err(),
            "FromStr and TryFrom agree on the invalid value"
        );
    }

    /// The `Display` output of `T` re-parses through `FromStr` to the
    /// same value, the text accessor returns the parsed text, and the
    /// `TryFrom<&str>` and `TryFrom<String>` conversions agree with
    /// `FromStr` on a valid and on an invalid value.
    fn round_trips_code<T, F>(valid: &str, invalid: &str, text: F)
    where
        T: Sized
            + fmt::Debug
            + fmt::Display
            + PartialEq
            + FromStr<Err = IdError>
            + for<'a> TryFrom<&'a str, Error = IdError>
            + TryFrom<String, Error = IdError>,
        F: Fn(&T) -> String,
    {
        let value = T::from_str(valid).expect("the valid value parses");
        assert_eq!(
            text(&value),
            valid,
            "the text accessor of the parsed {valid}"
        );
        let rendered = value.to_string();
        assert_eq!(rendered, valid, "the display output of the parsed {valid}");
        assert_eq!(
            T::from_str(&rendered).expect("the display output parses"),
            value,
            "the display output re-parses to the same value"
        );
        assert_eq!(
            T::try_from(rendered.as_str()).expect("the display output converts"),
            value,
            "TryFrom agrees with FromStr on the valid value"
        );
        assert_eq!(
            T::try_from(valid.to_string()).expect("the owned text converts"),
            value,
            "TryFrom<String> agrees with FromStr on the valid value"
        );
        assert_eq!(
            T::from_str(invalid).map_err(|e| e.to_string()),
            T::try_from(invalid.to_string()).map_err(|e| e.to_string()),
            "TryFrom<String> reports the same error as FromStr"
        );
    }

    /// Every newtype of the module round-trips through its `Display`
    /// output, its text accessor returns the parsed text, and its
    /// `TryFrom<&str>` and `TryFrom<String>` conversions agree with
    /// its `FromStr` implementation.
    #[test]
    fn display_output_parses_back_and_try_from_agrees_with_from_str() {
        round_trips::<PartitaIva, _>("02222222222", "01234567890", |v| v.as_str().to_string());
        round_trips::<CodiceFiscale, _>("MRALNE80E05H501C", "MRALNE80E05H501D", |v| {
            v.as_str().to_string()
        });
        round_trips_code::<Cap, _>("00100", "0010", |v| v.as_str().to_string());
        round_trips_code::<Provincia, _>("TO", "XX", |v| v.code().to_string());
        round_trips::<CodiceDestinatario, _>("ABC1234", "AB1", |v| v.as_str().to_string());
        round_trips::<ProgressivoInvio, _>("00001", "123456", |v| v.as_str().to_string());
        round_trips::<Bic, _>("DEUTDEFF501", "DEUTDEFF5011", |v| v.as_str().to_string());
        round_trips::<ForeignTaxId, _>("DE998877665", "de998877665", |v| v.as_str().to_string());
        round_trips::<Telefono, _>("+390612345", "0612", |v| v.as_str().to_string());
        round_trips::<Pec, _>("pec@agenzia.it", "not-an-email", |v| v.as_str().to_string());
        round_trips::<Email, _>("contact@example.com", "a@b.c", |v| v.as_str().to_string());
        round_trips::<Iban, _>(
            "IT60X0542811101000000123456",
            "IT60X0542811101000000123457",
            |v| v.as_str().to_string(),
        );
        round_trips_code::<Aliquota, _>("22.00", "100.01", ToString::to_string);
        round_trips_code::<ExchangeRate, _>("1.0800", "-1.08", ToString::to_string);
        round_trips::<RiferimentoNormativo, _>("ART. 7-TER DPR 633/72", &"a".repeat(101), |v| {
            v.as_str().to_string()
        });
        round_trips::<Numero, _>("2025-0001", "ABCDEFGHIJK", |v| v.as_str().to_string());
        round_trips::<IdDocumento, _>("FAT2025", "012345678901234567890", |v| {
            v.as_str().to_string()
        });
        round_trips::<Causale, _>("CAUSALE DELLA FATTURA", "CAUSALE\n", |v| {
            v.as_str().to_string()
        });
        round_trips::<Description, _>("DESCRIZIONE ARTICOLO", &"a".repeat(1001), |v| {
            v.as_str().to_string()
        });
        round_trips::<Denomination, _>("AZIENDA SRL", &"a".repeat(81), |v| v.as_str().to_string());
        round_trips::<PersonName, _>("MARIO", &"a".repeat(61), |v| v.as_str().to_string());
        round_trips::<Indirizzo, _>("VIA ROMA 1", &"a".repeat(61), |v| v.as_str().to_string());
        round_trips::<Comune, _>("ROMA", &"a".repeat(61), |v| v.as_str().to_string());
        round_trips::<UnitaMisura, _>("PZ", "01234567890", |v| v.as_str().to_string());
        round_trips::<PostalCode, _>("10100", "s-09011", |v| v.as_str().to_string());
        round_trips_code::<EuCountry, _>("GR", "EL", |v| v.code().to_string());
        round_trips_code::<Natura, _>("N2.1", "N2", |v| v.code().to_string());
        round_trips_code::<RegimeFiscale, _>("RF01", "RF03", |v| v.code().to_string());
    }

    /// The decimal view of a rate keeps the parsed value at its
    /// rescaled precision.
    #[test]
    fn rate_decimal_views_keep_the_parsed_value() {
        let rate = "22.00".parse::<Aliquota>().expect("22.00 is a valid rate");
        assert_eq!(rate.as_decimal(), Decimal::new(2200, 2));
        let exchange = "1.0800"
            .parse::<ExchangeRate>()
            .expect("1.0800 is a valid exchange rate");
        assert_eq!(exchange.as_decimal(), Decimal::new(10800, 4));
    }

    /// The `ISO 3166-1` alpha-2 codes of the 27 EU member states, in the
    /// order of `EuCountry::ALL`.
    const EU_MEMBERS: [&str; 27] = [
        "AT", "BE", "BG", "CY", "CZ", "DE", "DK", "EE", "ES", "FI", "FR", "GR", "HR", "HU", "IE",
        "IT", "LT", "LU", "LV", "MT", "NL", "PL", "PT", "RO", "SE", "SI", "SK",
    ];

    /// The `RegimeFiscale` codes of the XSD enumeration, in the order of
    /// `RegimeFiscale::ALL`.
    const REGIME_FISCALI: [&str; 19] = [
        "RF01", "RF02", "RF04", "RF05", "RF06", "RF07", "RF08", "RF09", "RF10", "RF11", "RF12",
        "RF13", "RF14", "RF15", "RF16", "RF17", "RF18", "RF19", "RF20",
    ];

    /// The current `Natura` codes of the schema, the retired generic
    /// `N2`, `N3` and `N6` aside, in the order of `Natura::ALL`.
    const NATURA_CODES: [&str; 21] = [
        "N1", "N2.1", "N2.2", "N3.1", "N3.2", "N3.3", "N3.4", "N3.5", "N3.6", "N4", "N5", "N6.1",
        "N6.2", "N6.3", "N6.4", "N6.5", "N6.6", "N6.7", "N6.8", "N6.9", "N7",
    ];

    /// Every variant of `EuCountry::ALL` carries its member state code
    /// and re-parses from it, and a non-member code is refused.
    #[test]
    fn eu_country_all_covers_the_eu() {
        let codes: Vec<&str> = EuCountry::ALL
            .iter()
            .map(|country| country.code())
            .collect();
        assert_eq!(
            codes, EU_MEMBERS,
            "ALL carries the 27 member states, one per code"
        );
        for country in EuCountry::ALL {
            assert_eq!(
                country.code().parse::<EuCountry>(),
                Ok(country),
                "the code of {} re-parses to itself",
                country.code()
            );
        }
        assert_eq!(
            "NO".parse::<EuCountry>(),
            Err(IdError::EuCountry("NO".to_string())),
            "Norway is not a member state"
        );
    }

    /// Every variant of `RegimeFiscale::ALL` carries its enumeration
    /// code and re-parses from it, and a code outside the enumeration
    /// is refused.
    #[test]
    fn regime_fiscale_all_covers_the_xsd_enumeration() {
        let codes: Vec<&str> = RegimeFiscale::ALL
            .iter()
            .map(|regime| regime.code())
            .collect();
        assert_eq!(
            codes, REGIME_FISCALI,
            "ALL carries the XSD enumeration, one code per variant"
        );
        for regime in RegimeFiscale::ALL {
            assert_eq!(
                regime.code().parse::<RegimeFiscale>(),
                Ok(regime),
                "the code of {} re-parses to itself",
                regime.code()
            );
        }
        assert_eq!(
            "RF03".parse::<RegimeFiscale>(),
            Err(IdError::RegimeFiscale("RF03".to_string())),
            "RF03 is outside the XSD enumeration"
        );
    }

    /// Every variant of `Natura::ALL` carries its code and re-parses from
    /// it, and the retired generic `N2` is refused.
    #[test]
    fn natura_all_covers_the_current_codes() {
        let codes: Vec<&str> = Natura::ALL.iter().map(|natura| natura.code()).collect();
        assert_eq!(
            codes, NATURA_CODES,
            "ALL carries the current codes, one per variant"
        );
        for natura in Natura::ALL {
            assert_eq!(
                natura.code().parse::<Natura>(),
                Ok(natura),
                "the code of {} re-parses to itself",
                natura.code()
            );
        }
        assert_eq!(
            "N2".parse::<Natura>(),
            Err(IdError::Natura("N2".to_string())),
            "the generic N2 is retired"
        );
    }

    /// Every variant of `Provincia::ALL` carries its census code and
    /// re-parses from it, and a code outside the census is refused.
    #[test]
    fn provincia_all_covers_the_census() {
        assert_eq!(Provincia::ALL.len(), 110, "the census carries 110 codes");
        let mut codes = std::collections::BTreeSet::new();
        for provincia in Provincia::ALL {
            codes.insert(provincia.code());
            assert_eq!(
                provincia.code().parse::<Provincia>(),
                Ok(provincia),
                "the code of {} re-parses to itself",
                provincia.code()
            );
        }
        assert_eq!(codes.len(), Provincia::ALL.len(), "no census code repeats");
        assert_eq!(
            "ZZ".parse::<Provincia>(),
            Err(IdError::Provincia("ZZ".to_string())),
            "ZZ is outside the census"
        );
    }
}
