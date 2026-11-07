//! The scale-preserving decimal type that the generated code uses for `xs:decimal`.

use rust_decimal::{Decimal, Error};
use std::fmt::Display;
use std::str::FromStr;

/// A decimal number that keeps the scale of its text form, so that `5.00`
/// survives a read and a write as `5.00` and not as `5`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
pub struct Amount(pub Decimal);

impl Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Amount> for String {
    fn from(value: Amount) -> Self {
        value.to_string()
    }
}

impl FromStr for Amount {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Decimal::from_str(s).map(Self)
    }
}

/// The `Amount` of the typed construction layer, which funnels the
/// conversion into the `FromStr` parse and reports the failure as
/// [`crate::ids::IdError::Amount`].
impl TryFrom<&str> for Amount {
    type Error = crate::ids::IdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
            .map_err(|e: Error| crate::ids::IdError::Amount(e.to_string()))
    }
}

/// The `Amount` of the typed construction layer, from an owned string.
impl TryFrom<String> for Amount {
    type Error = crate::ids::IdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(s.as_str())
    }
}

/// The already parsed decimal, which the wrapper never refuses.
impl From<Decimal> for Amount {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl xsd_parser_types::quick_xml::DeserializeBytesFromStr for Amount {}
impl xsd_parser_types::quick_xml::SerializeBytesToString for Amount {}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing case"
)]
mod tests {
    use super::*;

    /// A decimal keeps the scale of the text it was read from.
    #[test]
    fn amount_keeps_its_scale() {
        for text in ["123.45", "0.00", "5", "5.00", "1234567.89123456"] {
            let amount = text.parse::<Amount>().expect("a valid decimal");
            assert_eq!(amount.to_string(), text, "round trip of {text}");
        }
    }

    /// Invalid decimal text is rejected.
    #[test]
    fn amount_rejects_invalid_text() {
        for text in ["", "abc", "1.2.3"] {
            assert!(text.parse::<Amount>().is_err(), "{text} must be rejected");
        }
    }

    /// The `Amount` conversion of the typed layer reports the parse
    /// failure as `IdError::Amount` and reads the owned string through
    /// the same parse.
    #[test]
    fn amount_try_from_reports_the_id_error() {
        assert!(matches!(
            Amount::try_from("abc"),
            Err(crate::ids::IdError::Amount(..))
        ));
        assert_eq!(
            Amount::try_from("123.45".to_string()).expect("a valid decimal"),
            "123.45".parse::<Amount>().expect("a valid decimal")
        );
    }

    /// The `From` conversion of an already parsed decimal keeps the value
    /// and agrees with the parse of the same text.
    #[test]
    fn the_from_conversion_keeps_the_decimal() {
        let amount = Amount::from(Decimal::new(12345, 2));
        assert_eq!(amount.0, Decimal::new(12345, 2));
        assert_eq!(amount, "123.45".parse::<Amount>().expect("a valid decimal"));
    }
}
