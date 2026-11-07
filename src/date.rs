//! The date and date-time types that the generated code uses for `xs:date`
//! and `xs:dateTime`.

use jiff::Timestamp;
use jiff::civil::Date as CivilDate;
use jiff::fmt::temporal::Pieces;
use jiff::tz::{Offset, TimeZone};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// A calendar date in the `YYYY-MM-DD` lexical form of `xs:date`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(pub CivilDate);

impl Display for Date {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Date {
    type Err = jiff::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let date = CivilDate::from_str(s).map(Self)?;
        // The civil parser also accepts date-time and offset forms; the
        // exact `YYYY-MM-DD` lexical form is the only one that round-trips.
        if date.0.to_string() != s {
            return Err(jiff::Error::from_args(format_args!(
                "not a plain YYYY-MM-DD date: {s}"
            )));
        }
        Ok(date)
    }
}

/// The `Date` of the typed construction layer, which funnels the
/// conversion into the `FromStr` parse and reports the failure as
/// [`crate::ids::IdError::Date`].
impl TryFrom<&str> for Date {
    type Error = crate::ids::IdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
            .map_err(|e: jiff::Error| crate::ids::IdError::Date(e.to_string()))
    }
}

/// The `Date` of the typed construction layer, from an owned string.
impl TryFrom<String> for Date {
    type Error = crate::ids::IdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(s.as_str())
    }
}

impl xsd_parser_types::quick_xml::DeserializeBytesFromStr for Date {}
impl xsd_parser_types::quick_xml::SerializeBytesToString for Date {}

/// The `DateTime` of the typed construction layer, which funnels the
/// conversion into the `FromStr` parse and reports the failure as
/// [`crate::ids::IdError::DateTime`].
impl TryFrom<&str> for DateTime {
    type Error = crate::ids::IdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
            .map_err(|e: jiff::Error| crate::ids::IdError::DateTime(e.to_string()))
    }
}

/// The `DateTime` of the typed construction layer, from an owned string.
impl TryFrom<String> for DateTime {
    type Error = crate::ids::IdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(s.as_str())
    }
}

/// A date and time in the lexical form of `xs:dateTime`.
///
/// The fractional seconds and the UTC offset of the text form are preserved
/// when they are present.
///
/// A text form without an offset, such as `2014-01-01T00:00:00`, is
/// interpreted as UTC and written back without an offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime {
    /// The instant this value refers to.
    pub instant: Timestamp,
    /// The UTC offset of the text form, if it carried one.
    pub offset: Option<Offset>,
}

impl Display for DateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let offset = self.offset.unwrap_or(Offset::UTC);
        let wall = offset.to_datetime(self.instant);
        write!(f, "{}", wall.strftime("%Y-%m-%dT%H:%M:%S"))?;
        let subsec = wall.subsec_nanosecond();
        if subsec != 0 {
            write!(f, ".{}", format!("{subsec:09}").trim_end_matches('0'))?;
        }
        if let Some(offset) = self.offset {
            if offset == Offset::UTC {
                return f.write_str("Z");
            }
            let secs = offset.seconds().unsigned_abs();
            let sign = if offset.is_negative() { "-" } else { "+" };
            write!(f, "{sign}{:02}:{:02}", secs / 3600, secs % 3600 / 60)
        } else {
            Ok(())
        }
    }
}

impl FromStr for DateTime {
    type Err = jiff::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // The date is the extended YYYY-MM-DD form, so byte 10 is the time
        // separator and a date without a time part cannot parse.
        if s.as_bytes().get(10) != Some(&b'T') {
            return Err(jiff::Error::from_args(format_args!(
                "a date-time needs a T separator: {s}"
            )));
        }
        let pieces = Pieces::parse(s)?;
        let Some(time) = pieces.time() else {
            return Err(jiff::Error::from_args(format_args!(
                "a date-time needs a time part: {s}"
            )));
        };
        if pieces.time_zone_annotation().is_some() {
            return Err(jiff::Error::from_args(format_args!(
                "a date-time cannot carry a time zone annotation: {s}"
            )));
        }
        let offset = match pieces.to_numeric_offset() {
            Some(offset) => Some(check_offset(s, offset)?),
            None => None,
        };
        let wall = pieces.date().to_datetime(time);
        let instant = wall
            .to_zoned(TimeZone::fixed(offset.unwrap_or(Offset::UTC)))?
            .timestamp();
        Ok(Self { instant, offset })
    }
}

impl xsd_parser_types::quick_xml::DeserializeBytesFromStr for DateTime {}
impl xsd_parser_types::quick_xml::SerializeBytesToString for DateTime {}

/// The `xs:dateTime` offset is `Z` or `±HH:MM` in `-14:00` to `+14:00`; the
/// temporal parser also accepts the basic and sub-minute offset forms.
fn check_offset(s: &str, offset: Offset) -> Result<Offset, jiff::Error> {
    // The time part is digits, colons and dots, so the offset is the first
    // `Z`, `z`, `+` or `-` after it.
    let Some(start) = s[11..]
        .bytes()
        .position(|b| matches!(b, b'Z' | b'z' | b'+' | b'-'))
        .map(|i| i + 11)
    else {
        return Err(jiff::Error::from_args(format_args!(
            "the offset of {s} is missing"
        )));
    };
    let lexical = match &s[start..] {
        "Z" | "z" => true,
        text => {
            text.len() == 6
                && matches!(text.as_bytes()[0], b'+' | b'-')
                && text.as_bytes()[1].is_ascii_digit()
                && text.as_bytes()[2].is_ascii_digit()
                && text.as_bytes()[3] == b':'
                && text.as_bytes()[4].is_ascii_digit()
                && text.as_bytes()[5].is_ascii_digit()
        }
    };
    if !lexical {
        return Err(jiff::Error::from_args(format_args!(
            "the offset of a date-time is Z or ±HH:MM: {s}"
        )));
    }
    // xs:dateTime allows offsets only from -14:00 to +14:00.
    if offset.seconds().unsigned_abs() > 14 * 3600 {
        return Err(jiff::Error::from_args(format_args!(
            "offset out of range: {s}"
        )));
    }
    Ok(offset)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing case"
)]
mod tests {
    use super::*;

    /// A date round-trips its `YYYY-MM-DD` lexical form.
    #[test]
    fn date_round_trips() {
        for text in ["2014-11-20", "2026-01-05", "0001-01-01"] {
            let date = text.parse::<Date>().expect("a valid xs:date");
            assert_eq!(date.to_string(), text, "round trip of {text}");
        }
    }

    /// Invalid date text is rejected.
    #[test]
    fn date_rejects_invalid_text() {
        for text in ["2014-02-30", "2014-11-20T00:00:00", ""] {
            assert!(text.parse::<Date>().is_err(), "{text} must be rejected");
        }
    }

    /// A date-time keeps its fractional seconds and offset designator.
    #[test]
    fn datetime_keeps_its_lexical_form() {
        let cases = [
            "2014-11-20T10:30:00Z",
            "2014-11-20T10:30:00+02:00",
            "2014-11-20T10:30:00-05:30",
            "2014-11-20T10:30:00",
            "2014-11-20T10:30:00.123456789+02:00",
            "2014-11-20T10:30:00.5Z",
        ];
        for text in cases {
            let parsed = text.parse::<DateTime>().expect("a valid xs:dateTime");
            assert_eq!(parsed.to_string(), text, "round trip of {text}");
        }
    }

    /// The lowercase `z` designator is normalised to `Z` on the way out,
    /// and the fractional seconds are written trimmed of trailing zeros.
    #[test]
    fn datetime_normalises_lower_z() {
        let parsed = "2014-11-20T10:30:00z"
            .parse::<DateTime>()
            .expect("a valid xs:dateTime");
        assert_eq!(parsed.to_string(), "2014-11-20T10:30:00Z");
    }

    /// Fractional seconds are written in their trimmed form.
    #[test]
    fn datetime_trims_fractional_seconds() {
        let parsed = "2014-11-20T10:30:00.500Z"
            .parse::<DateTime>()
            .expect("a valid xs:dateTime");
        assert_eq!(parsed.to_string(), "2014-11-20T10:30:00.5Z");
    }

    /// Invalid date-time text is rejected.
    #[test]
    fn datetime_rejects_invalid_text() {
        let cases = [
            "2014-11-20",
            "2014-11-20T25:00:00Z",
            "2014-11-20T10:30:00+24:00",
            "2014-11-20T10:30:00Z+02:00",
        ];
        for text in cases {
            assert!(text.parse::<DateTime>().is_err(), "{text} must be rejected");
        }
    }

    /// The `xs:dateTime` offset range is `-14:00` to `+14:00`, which the
    /// boundary offsets read and write back.
    #[test]
    fn datetime_offset_range() {
        let cases = [
            "2014-11-20T10:30:00+14:00",
            "2014-11-20T10:30:00-14:00",
            "2014-11-20T10:30:00+05:30",
        ];
        for text in cases {
            let parsed = text.parse::<DateTime>().expect("a valid xs:dateTime");
            assert_eq!(parsed.to_string(), text, "round trip of {text}");
        }
        let cases = [
            "2014-11-20T10:30:00+14:01",
            "2014-11-20T10:30:00-14:01",
            "2014-11-20T10:30:00+23:00",
            "2014-11-20T10:30:00-23:59",
            "2014-11-20T10:30:00+05:99",
        ];
        for text in cases {
            assert!(text.parse::<DateTime>().is_err(), "{text} must be rejected");
        }
    }

    /// The `Date` conversion of the typed layer reports the parse failure
    /// as `IdError::Date` and reads the owned string through the same
    /// parse.
    #[test]
    fn date_try_from_reports_the_id_error() {
        assert!(matches!(
            Date::try_from("2014-02-30"),
            Err(crate::ids::IdError::Date(..))
        ));
        assert_eq!(
            Date::try_from("2026-11-04".to_string()).expect("a valid date"),
            "2026-11-04".parse::<Date>().expect("a valid date")
        );
    }

    /// The `DateTime` conversion of the typed layer reports the parse
    /// failure as `IdError::DateTime` and reads the owned string through
    /// the same parse.
    #[test]
    fn datetime_try_from_reports_the_id_error() {
        assert!(matches!(
            DateTime::try_from("2014-11-20"),
            Err(crate::ids::IdError::DateTime(..))
        ));
        assert_eq!(
            DateTime::try_from("2014-11-20T10:30:00Z".to_string()).expect("a valid date-time"),
            "2014-11-20T10:30:00Z"
                .parse::<DateTime>()
                .expect("a valid date-time")
        );
    }

    /// Offsets outside the `Z` and `±HH:MM` lexical form are rejected, as
    /// are time zone annotations and a missing time part.
    #[test]
    fn datetime_rejects_offsets_outside_the_lexical_form() {
        let cases = [
            "2014-11-20T10:30:00+5:30",
            "2014-11-20T10:30:00+05:3x",
            "2014-11-20T10:30:00+0530",
            "2014-11-20T10:30:00-0530",
            "2014-11-20T10:30:00+05",
            "2014-11-20T10:30:00+05:30:45",
            "2014-11-20T10:30:00[Europe/Rome]",
            "2014-11-20T10:30:00+02:00[+02:00]",
            "2014-11-20T",
        ];
        for text in cases {
            assert!(text.parse::<DateTime>().is_err(), "{text} must be rejected");
        }
    }
}
