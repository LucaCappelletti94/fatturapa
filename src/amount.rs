//! The scale-preserving decimal type that the generated code uses for `xs:decimal`.

use rust_decimal::{Decimal, Error};
use std::fmt::Display;
use std::str::FromStr;

/// A decimal number that keeps the scale of its text form, so that `5.00`
/// survives a read and a write as `5.00` and not as `5`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount(pub Decimal);

impl Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Amount {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Decimal::from_str(s).map(Self)
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
}
