//! The `SDI` file names that the exchange checks as errors `00001` to
//! `00003` of the controls list.
//!
//! An invoice file is named `IT01234567890_00001.xml` in the `XAdES` form and
//! `IT01234567890_00001.xml.p7m` in the `CAdES` form. The name holds the
//! country code, the unique identifier of the sender and a file progressive
//! that the sender assigns to keep every name it sends distinct, which is
//! error `00002` when it is not. Error `00003` is the size of the file, which
//! the transmission channel decides and this module does not see.
//!
//! The `SDI` receipts and notifications are named after the received invoice
//! file without its extension, with the message type and a message
//! progressive appended, as in `IT01234567890_00001_RC_001.xml`. When the
//! received file name is not conformant and longer than 36 characters the
//! `SDI` truncates it in the rejection notification, so the invoice part of
//! a real notification may not parse.
//!
//! The rules come from the nomenclature sections of the `SDI` technical
//! specification. The country code is an `ISO 3166-1` alpha-2 code in upper
//! case. The identifier is 11 to 16 characters long for the country `IT` and
//! 2 to 28 characters otherwise. The file progressive is alphanumeric with
//! at most 5 characters, and the message progressive at most 3. The
//! identifier is checked as alphanumeric for the same alphabet, since the
//! separators of the name are underscores and every example in the
//! specification uses letters and digits.

use std::fmt;
use std::str::FromStr;

/// Why an `SDI` file name is invalid.
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum NameError {
    /// The country code is not two upper-case letters.
    #[error("the country code {0:?} is not an ISO 3166-1 alpha-2 code in upper case")]
    Country(String),

    /// The sender identifier is outside the length limits of its country.
    #[error("the identifier {name:?} is not {min} to {max} alphanumeric characters")]
    Identifier {
        /// The identifier that failed the check.
        name: String,
        /// The minimum length, in characters.
        min: usize,
        /// The maximum length, in characters.
        max: usize,
    },

    /// The file progressive is empty, too long, or not alphanumeric.
    #[error("the file progressive {0:?} is not 1 to 5 alphanumeric characters")]
    Progressive(String),

    /// The extension is not `.xml` or `.xml.p7m` where an invoice file is expected.
    #[error("the extension of {0:?} is not .xml or .xml.p7m")]
    Extension(String),

    /// The extension of a receipt or notification name is not `.xml`.
    #[error("the extension of {0:?} is not .xml")]
    MessageExtension(String),

    /// The message type is not one of the `SDI` receipt and notification codes.
    #[error("the message type {0:?} is not one of RC NS MC NE MT EC SE DT AT")]
    MessageKind(String),

    /// The message progressive is empty, too long, or not alphanumeric.
    #[error("the message progressive {0:?} is not 1 to 3 alphanumeric characters")]
    MessageProgressive(String),

    /// The name has the wrong shape, such as a missing or repeated separator.
    #[error("the name {0:?} is not the expected shape of the SDI file name")]
    Shape(String),
}

/// The type code of an `SDI` receipt or notification, per the nomenclature
/// of the technical specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
pub enum MessageKind {
    /// The delivery receipt, `RC`.
    Rc,
    /// The rejection notification, `NS`.
    Ns,
    /// The failed delivery notification, `MC`.
    Mc,
    /// The outcome notification for the private recipient, `NE`.
    Ne,
    /// The metadata file, `MT`.
    Mt,
    /// The outcome notification for the public administration recipient, `EC`.
    Ec,
    /// The rejection of a recipient outcome notification, `SE`.
    Se,
    /// The deadline passage notification, `DT`.
    Dt,
    /// The transmission certificate with impossible delivery, `AT`.
    At,
}

impl MessageKind {
    /// The two-letter code of the message type.
    const fn code(self) -> &'static str {
        match self {
            Self::Rc => "RC",
            Self::Ns => "NS",
            Self::Mc => "MC",
            Self::Ne => "NE",
            Self::Mt => "MT",
            Self::Ec => "EC",
            Self::Se => "SE",
            Self::Dt => "DT",
            Self::At => "AT",
        }
    }
}

impl fmt::Display for MessageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for MessageKind {
    type Err = NameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "RC" => Ok(Self::Rc),
            "NS" => Ok(Self::Ns),
            "MC" => Ok(Self::Mc),
            "NE" => Ok(Self::Ne),
            "MT" => Ok(Self::Mt),
            "EC" => Ok(Self::Ec),
            "SE" => Ok(Self::Se),
            "DT" => Ok(Self::Dt),
            "AT" => Ok(Self::At),
            _ => Err(NameError::MessageKind(s.to_string())),
        }
    }
}

impl TryFrom<&str> for MessageKind {
    type Error = NameError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl TryFrom<String> for MessageKind {
    type Error = NameError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl From<MessageKind> for String {
    fn from(value: MessageKind) -> Self {
        value.to_string()
    }
}

/// The name of an invoice file sent to the `SDI`.
///
/// The signed flag picks between the `.xml` extension of the `XAdES` form
/// and the `.xml.p7m` extension of the `CAdES` form. The name of a signed
/// file is the name of the unsigned file with `.p7m` appended.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
pub struct InvoiceName {
    /// The country code, as in `IT`.
    country: String,
    /// The `VAT` number or the fiscal code of the subject.
    identifier: String,
    /// The progressive number of the invoice.
    progressive: String,
    /// Whether the name is the signed `.xml.p7m` form.
    signed: bool,
}

impl InvoiceName {
    /// Builds the name of an unsigned invoice file, in the `.xml` form.
    ///
    /// # Errors
    ///
    /// Fails with [`NameError`] when the country code, the identifier or the
    /// progressive is outside the rules of the nomenclature.
    pub fn new(
        country: impl Into<String>,
        identifier: impl Into<String>,
        progressive: impl Into<String>,
    ) -> Result<Self, NameError> {
        let country = country.into();
        let identifier = identifier.into();
        let progressive = progressive.into();
        check_country(&country)?;
        let identifier = CheckedIdentifier::check(&country, &identifier)?;
        let progressive = check_progressive(&progressive)?;
        Ok(Self {
            country,
            identifier: identifier.into_owned(),
            progressive,
            signed: false,
        })
    }

    /// The same name in the signed `.xml.p7m` form.
    #[must_use]
    pub const fn as_signed(mut self) -> Self {
        self.signed = true;
        self
    }

    /// The country code, as in `IT`.
    #[must_use]
    pub fn country(&self) -> &str {
        &self.country
    }

    /// The unique identifier of the sender.
    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// The file progressive assigned by the sender.
    #[must_use]
    pub fn progressive(&self) -> &str {
        &self.progressive
    }

    /// Whether the name is in the signed `.xml.p7m` form.
    #[must_use]
    pub const fn signed(&self) -> bool {
        self.signed
    }

    /// The name without its extension, the part an `SDI` receipt or
    /// notification name is built on.
    #[must_use]
    pub fn base(&self) -> String {
        format!(
            "{}{}_{progressive}",
            self.country,
            self.identifier,
            progressive = self.progressive
        )
    }

    /// The extension of the name, `.xml` or `.xml.p7m`.
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        if self.signed { ".xml.p7m" } else { ".xml" }
    }
}

impl fmt::Display for InvoiceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.base(), self.extension())
    }
}

impl FromStr for InvoiceName {
    type Err = NameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (base, signed) = if let Some(base) = s.strip_suffix(".xml.p7m") {
            (base, true)
        } else if let Some(base) = s.strip_suffix(".xml") {
            (base, false)
        } else {
            return Err(NameError::Extension(s.to_string()));
        };
        let (country, identifier, progressive) = parse_base(base, s)?;
        Ok(Self {
            country,
            identifier,
            progressive,
            signed,
        })
    }
}

impl TryFrom<&str> for InvoiceName {
    type Error = NameError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl TryFrom<String> for InvoiceName {
    type Error = NameError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl From<InvoiceName> for String {
    fn from(value: InvoiceName) -> Self {
        value.to_string()
    }
}

/// The name of an `SDI` receipt or notification file.
///
/// The invoice part holds the country code, the identifier and the file
/// progressive of the received invoice file, without its extension. The
/// signed flag of that invoice file does not appear in the name, since the
/// extension of the received file is the part the message type and the
/// message progressive replace.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
pub struct MessageName {
    /// The invoice file the message refers to.
    invoice: InvoiceName,
    /// The kind of the `SDI` message.
    kind: MessageKind,
    /// The progressive number of the message.
    progressive: String,
}

impl MessageName {
    /// Builds the name of a receipt or notification file.
    ///
    /// The signed flag of `invoice` is not part of the name, so either form
    /// of the invoice file yields the same message name.
    ///
    /// # Errors
    ///
    /// Fails with [`NameError`] when the message progressive is outside the
    /// rules of the nomenclature.
    pub fn new(
        invoice: InvoiceName,
        kind: MessageKind,
        progressive: impl Into<String>,
    ) -> Result<Self, NameError> {
        Ok(Self {
            invoice,
            kind,
            progressive: check_progressive3(&progressive.into())?,
        })
    }

    /// The name of the received invoice file the message refers to, in the
    /// unsigned form.
    #[must_use]
    pub const fn invoice(&self) -> &InvoiceName {
        &self.invoice
    }

    /// The type of the message.
    #[must_use]
    pub const fn kind(&self) -> MessageKind {
        self.kind
    }

    /// The message progressive, which distinguishes the notifications about
    /// the same file.
    #[must_use]
    pub fn progressive(&self) -> &str {
        &self.progressive
    }
}

impl fmt::Display for MessageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}_{}_{}.xml",
            self.invoice.base(),
            self.kind,
            self.progressive
        )
    }
}

impl FromStr for MessageName {
    type Err = NameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let base = s
            .strip_suffix(".xml")
            .ok_or_else(|| NameError::MessageExtension(s.to_string()))?;
        let (rest, progressive) = base
            .rsplit_once('_')
            .ok_or_else(|| NameError::Shape(s.to_string()))?;
        let progressive =
            check_progressive3(progressive).map_err(|_| NameError::Shape(s.to_string()))?;
        let (invoice_base, kind) = rest
            .rsplit_once('_')
            .ok_or_else(|| NameError::Shape(s.to_string()))?;
        let kind = kind.parse::<MessageKind>()?;
        let (country, identifier, invoice_progressive) = parse_base(invoice_base, s)?;
        Ok(Self {
            invoice: InvoiceName {
                country,
                identifier,
                progressive: invoice_progressive,
                signed: false,
            },
            kind,
            progressive,
        })
    }
}

impl TryFrom<&str> for MessageName {
    type Error = NameError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl TryFrom<String> for MessageName {
    type Error = NameError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl From<MessageName> for String {
    fn from(value: MessageName) -> Self {
        value.to_string()
    }
}

/// Checks the two-letter country code.
fn check_country(country: &str) -> Result<(), NameError> {
    if country.len() == 2 && country.chars().all(|c| c.is_ascii_uppercase()) {
        Ok(())
    } else {
        Err(NameError::Country(country.to_string()))
    }
}

/// The checked identifier, still borrowed.
struct CheckedIdentifier<'a>(&'a str);

impl<'a> CheckedIdentifier<'a> {
    /// Checks the identifier length for the country `country`.
    fn check(country: &str, identifier: &'a str) -> Result<Self, NameError> {
        let (min, max) = if country == "IT" { (11, 16) } else { (2, 28) };
        let len = identifier.chars().count();
        if (min..=max).contains(&len) && alphanumeric(identifier) {
            Ok(Self(identifier))
        } else {
            Err(NameError::Identifier {
                name: identifier.to_string(),
                min,
                max,
            })
        }
    }

    /// The owned form.
    fn into_owned(self) -> String {
        self.0.to_string()
    }
}

/// Checks the file progressive, 1 to 5 alphanumeric characters, shared
/// with the `ProgressivoInvio` of the typed construction.
pub(crate) fn check_progressive(progressive: &str) -> Result<String, NameError> {
    if (1..=5).contains(&progressive.chars().count()) && alphanumeric(progressive) {
        Ok(progressive.to_string())
    } else {
        Err(NameError::Progressive(progressive.to_string()))
    }
}

/// Checks the message progressive, 1 to 3 alphanumeric characters.
fn check_progressive3(progressive: &str) -> Result<String, NameError> {
    if (1..=3).contains(&progressive.chars().count()) && alphanumeric(progressive) {
        Ok(progressive.to_string())
    } else {
        Err(NameError::MessageProgressive(progressive.to_string()))
    }
}

/// Splits a base name into country, identifier and progressive.
fn parse_base(base: &str, original: &str) -> Result<(String, String, String), NameError> {
    let (left, progressive) = base
        .rsplit_once('_')
        .ok_or_else(|| NameError::Shape(original.to_string()))?;
    // The byte-2 split fails on a one character prefix and inside a character.
    let (country, identifier) = left
        .split_at_checked(2)
        .ok_or_else(|| NameError::Shape(original.to_string()))?;
    check_country(country)?;
    if identifier.is_empty() {
        return Err(NameError::Shape(original.to_string()));
    }
    let identifier = CheckedIdentifier::check(country, identifier)?;
    let progressive = check_progressive(progressive)?;
    Ok((country.to_string(), identifier.into_owned(), progressive))
}

/// Non-empty ASCII letters and digits only.
fn alphanumeric(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric())
}
