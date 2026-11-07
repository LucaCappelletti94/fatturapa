//! The `SDI` file names, valid and invalid, per the rules the exchange
//! checks as errors `00001` to `00003`.
#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

use fatturapa::filename::{InvoiceName, MessageKind, MessageName, NameError};

/// The name in the specification examples, unsigned.
const NAME: &str = "IT01234567890_00001.xml";

#[test]
fn invoice_name_round_trips() {
    for (name, signed) in [(NAME, false), ("IT01234567890_00001.xml.p7m", true)] {
        let parsed = name.parse::<InvoiceName>().expect("the name is valid");
        assert_eq!(parsed.country(), "IT");
        assert_eq!(parsed.identifier(), "01234567890");
        assert_eq!(parsed.progressive(), "00001");
        assert_eq!(parsed.signed(), signed);
        assert_eq!(parsed.base(), "IT01234567890_00001");
        assert_eq!(parsed.to_string(), name);
    }
}

#[test]
fn invoice_name_builds() {
    let name = InvoiceName::new("IT", "01234567890", "00001").expect("the name is valid");
    assert_eq!(name.to_string(), NAME);
    assert_eq!(name.as_signed().to_string(), "IT01234567890_00001.xml.p7m");
}

#[test]
fn identifier_lengths_by_country() {
    // An Italian VAT number is 11 characters and a tax code 16.
    for identifier in ["01234567890", "AAABBB99T99X999W"] {
        InvoiceName::new("IT", identifier, "00001")
            .expect("the identifier is within 11 to 16 characters");
    }
    // A foreign identifier is 2 to 28 characters.
    for identifier in ["12", "1234567890123456789012345678"] {
        let name = InvoiceName::new("FR", identifier, "00001")
            .expect("the identifier is within 2 to 28 characters");
        assert_eq!(name.country(), "FR");
    }
}

#[test]
fn identifier_lengths_outside_the_limits_are_rejected() {
    for identifier in ["0123456789", "012345678901234567"] {
        let error = InvoiceName::new("IT", identifier, "00001")
            .expect_err("the identifier is outside 11 to 16 characters");
        assert!(
            matches!(error, NameError::Identifier { .. }),
            "the error names the identifier, got {error:?}"
        );
    }
    for identifier in ["1", "12345678901234567890123456789"] {
        let error = InvoiceName::new("FR", identifier, "00001")
            .expect_err("the identifier is outside 2 to 28 characters");
        assert!(
            matches!(error, NameError::Identifier { .. }),
            "the error names the identifier, got {error:?}"
        );
    }
}

#[test]
fn identifier_must_be_alphanumeric() {
    for identifier in [
        "0123456789_0",
        "0123456789-0",
        "0123456789 0",
        "0123456789.0",
    ] {
        assert!(
            InvoiceName::new("IT", identifier, "00001").is_err(),
            "{identifier} is not alphanumeric"
        );
    }
}

#[test]
fn progressive_is_alphanumeric_up_to_five_characters() {
    for progressive in ["1", "00001", "AbC01"] {
        InvoiceName::new("IT", "01234567890", progressive)
            .expect("the progressive is 1 to 5 alphanumeric characters");
    }
    for progressive in ["", "000010", "000_1"] {
        assert!(
            InvoiceName::new("IT", "01234567890", progressive).is_err(),
            "{progressive:?} is not 1 to 5 alphanumeric characters"
        );
    }
}

#[test]
fn country_code_is_two_upper_case_letters() {
    for country in ["I0", "it", "i", "IT0"] {
        let error = InvoiceName::new(country, "01234567890", "00001")
            .expect_err("the country code is invalid");
        assert!(
            matches!(error, NameError::Country(_)),
            "{country:?} fails with the country error, got {error:?}"
        );
    }
    assert!(InvoiceName::new("FR", "12", "00001").is_ok());
}

#[test]
fn parse_rejects_wrong_extensions() {
    for name in [
        "IT01234567890_00001",
        "IT01234567890_00001.XML",
        "IT01234567890_00001.p7m",
        "IT01234567890_00001.xml ",
    ] {
        let error = name
            .parse::<InvoiceName>()
            .expect_err("the extension is not .xml or .xml.p7m");
        assert!(
            matches!(error, NameError::Extension(_)),
            "{name:?} fails with the extension error, got {error:?}"
        );
    }
}

#[test]
fn parse_rejects_wrong_shapes() {
    for name in [
        "IT01234567890.xml",
        "IT01234567890__00001.xml",
        "IT01234567890_00001_.xml",
        "T01234567890_00001.xml",
        "IT01234567890_00001_00001.xml",
    ] {
        assert!(
            name.parse::<InvoiceName>().is_err(),
            "{name:?} is not a valid invoice file name"
        );
    }
}

/// A base whose identifier is empty or starts in the middle of a character
/// is a shape error, and a one character identifier is the identifier error.
#[test]
fn parse_base_rejects_empty_or_unsplit_identifiers() {
    let error = "Ià1_00001.xml"
        .parse::<InvoiceName>()
        .expect_err("byte 2 of Ià1 is inside a character");
    assert!(
        matches!(error, NameError::Shape(_)),
        "the name fails with the shape error, got {error:?}"
    );
    for name in ["I_00001.xml", "IT_00001.xml"] {
        let error = name
            .parse::<InvoiceName>()
            .expect_err("the identifier is empty");
        assert!(
            matches!(error, NameError::Shape(_)),
            "{name:?} fails with the shape error, got {error:?}"
        );
    }
    let error = "IT1_00001.xml"
        .parse::<InvoiceName>()
        .expect_err("the identifier is one character");
    assert!(
        matches!(error, NameError::Identifier { .. }),
        "the error names the identifier, got {error:?}"
    );
}

/// Every message type parses, displays and round-trips.
#[test]
fn message_kinds_round_trip() {
    const KINDS: [(MessageKind, &str); 9] = [
        (MessageKind::Rc, "RC"),
        (MessageKind::Ns, "NS"),
        (MessageKind::Mc, "MC"),
        (MessageKind::Ne, "NE"),
        (MessageKind::Mt, "MT"),
        (MessageKind::Ec, "EC"),
        (MessageKind::Se, "SE"),
        (MessageKind::Dt, "DT"),
        (MessageKind::At, "AT"),
    ];
    for (kind, code) in KINDS {
        assert_eq!(kind.to_string(), code);
        assert_eq!(
            code.parse::<MessageKind>().expect("the code is valid"),
            kind
        );
    }
    for code in ["RX", "RC1", "rc", ""] {
        assert!(
            code.parse::<MessageKind>().is_err(),
            "{code:?} is not a message type"
        );
    }
}

/// The `TryFrom<&str>` impls of the three name types agree with their
/// `FromStr` impls on every valid and invalid form.
#[test]
fn try_from_str_agrees_with_from_str() {
    for code in ["RC", "NS", "MC", "NE", "MT", "EC", "SE", "DT", "AT"] {
        assert_eq!(MessageKind::try_from(code), code.parse::<MessageKind>());
    }
    for code in ["RX", "RC1", "rc", ""] {
        assert_eq!(MessageKind::try_from(code), code.parse::<MessageKind>());
    }

    for name in [
        NAME,
        "IT01234567890_00001.xml.p7m",
        "IT01234567890_00001",
        "it01234567890_00001.xml",
    ] {
        assert_eq!(InvoiceName::try_from(name), name.parse::<InvoiceName>());
    }

    for name in [
        "IT01234567890_00001_RC_001.xml",
        "IT01234567890_00001_RC_001",
        "IT01234567890_00001_RR_001.xml",
    ] {
        assert_eq!(MessageName::try_from(name), name.parse::<MessageName>());
    }
}

#[test]
fn message_name_round_trips() {
    for (name, kind, invoice, progressive) in [
        (
            "IT01234567890_00001_RC_001.xml",
            MessageKind::Rc,
            "IT01234567890_00001.xml",
            "001",
        ),
        (
            "IT01234567890_00001_NS_001.xml",
            MessageKind::Ns,
            "IT01234567890_00001.xml",
            "001",
        ),
        (
            "FR12_00001_MC_999.xml",
            MessageKind::Mc,
            "FR12_00001.xml",
            "999",
        ),
    ] {
        let parsed = name.parse::<MessageName>().expect("the name is valid");
        assert_eq!(parsed.kind(), kind);
        assert_eq!(parsed.invoice().to_string(), invoice);
        assert_eq!(parsed.progressive(), progressive);
        assert_eq!(parsed.to_string(), name);
    }
}

#[test]
fn message_name_builds() {
    let invoice = InvoiceName::new("IT", "01234567890", "00001").expect("the name is valid");
    let name = MessageName::new(invoice.as_signed(), MessageKind::Rc, "001")
        .expect("the progressive is valid");
    assert_eq!(name.to_string(), "IT01234567890_00001_RC_001.xml");
}

#[test]
fn message_name_rejects_bad_progressives() {
    for progressive in ["", "0001", "00_1"] {
        let invoice = InvoiceName::new("IT", "01234567890", "00001").expect("the name is valid");
        assert!(
            MessageName::new(invoice, MessageKind::Rc, progressive).is_err(),
            "{progressive:?} is not 1 to 3 alphanumeric characters"
        );
    }
}

#[test]
fn message_name_rejects_bad_shapes() {
    for name in [
        "IT01234567890_00001_RC_001.xml.p7m",
        "IT01234567890_00001_RC.xml",
        "IT01234567890_RC_001.xml",
        "IT01234567890_00001_RX_001.xml",
        "IT01234567890_00001_RC_0001.xml",
        "IT01234567890_00001_RC_001",
        "IT01234567890_001.xml",
        "IT01234567890.xml",
    ] {
        assert!(
            name.parse::<MessageName>().is_err(),
            "{name:?} is not a valid message file name"
        );
    }
}
