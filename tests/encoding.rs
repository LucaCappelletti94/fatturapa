//! Reading documents whose XML declaration names a non-UTF-8 `encoding`, and
//! rejecting the ones it cannot.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

#[path = "support/mod.rs"]
mod support;

use encoding_rs::Encoding;
use fatturapa::ids::{Provincia, RegimeFiscale};
use fatturapa::party::{Address, Seller};
use fatturapa::{Document, Error, FatturaElettronica, Invoice, Line, Recipient, VatTreatment};

/// The Italian seller carrying the `denomination` of `name`, the rest of the
/// invoice staying ASCII.
fn seller_with(name: &str) -> Seller {
    Seller::denomination(
        name.parse().expect("a valid denomination"),
        "02222222222".parse().expect("a valid partita IVA"),
        Address::italian(
            "VIA ROMA 1".parse().expect("a valid address"),
            "00100".parse().expect("a valid CAP"),
            "ROMA".parse().expect("a valid comune"),
            Provincia::Rm,
        ),
        RegimeFiscale::Rf01,
    )
}

/// A minimal domestic invoice, `seller` and one taxed line at 22 percent.
fn domestic_invoice(seller: &Seller) -> FatturaElettronica {
    let line = Line::builder()
        .description("OLIO E VARI")
        .quantity("1.00")
        .unit_price("10.00")
        .vat(VatTreatment::Taxed("22.00".parse().expect("a valid rate")))
        .build()
        .expect("the line builds");
    Invoice::domestic(
        seller,
        &support::buyer(),
        Recipient::Codice("ABC1234".parse().expect("a valid code")),
        line,
    )
    .progressivo("00001")
    .date("2026-11-04")
    .numero("00013")
    .build()
    .expect("the invoice passes every check")
}

/// Rewrites the `encoding` the declaration of the `UTF-8` document `utf8`
/// names to `label` and re-encodes the whole document as `label`.
fn reencode_as(utf8: &[u8], label: &str) -> Vec<u8> {
    let text = std::str::from_utf8(utf8).expect("the invoice serialises as UTF-8");
    let text = text.replace(r#"encoding="UTF-8""#, &format!(r#"encoding="{label}""#));
    Encoding::for_label(label.as_bytes())
        .expect("a WHATWG label")
        .encode(&text)
        .0
        .into_owned()
}

/// The `ISO-8859-1` bytes the declaration names decode, the
/// Latin-1-supplement name surviving the round-trip to the `UTF-8` form.
#[test]
fn reads_iso_8859_1() {
    let name = "Caffè Società Perché S.r.l.";
    let original = domestic_invoice(&seller_with(name));
    let reencoded = reencode_as(
        &original.to_xml_vec().expect("the invoice serialises"),
        "ISO-8859-1",
    );
    assert!(
        reencoded.contains(&0xE8),
        "the re-encoded document carries the ISO-8859-1 è byte"
    );
    assert!(
        !reencoded.windows(2).any(|w| w == b"\xC3\xA8"),
        "…not its two-byte UTF-8 form"
    );
    support::validate(&support::fpr12_schema(), &reencoded)
        .expect("the ISO-8859-1 document validates against the schema");
    let parsed =
        FatturaElettronica::from_xml_slice(&reencoded).expect("the ISO-8859-1 invoice reads");
    assert_eq!(parsed, original, "the ISO-8859-1 invoice round-trips");
    let written = parsed.to_xml_vec().expect("the invoice writes");
    support::validate(&support::fpr12_schema(), &written)
        .expect("the written invoice validates against the schema");
    assert!(
        written.windows(2).any(|w| w == b"\xC3\xA8"),
        "the written form re-encodes è as UTF-8"
    );
}

/// The `windows-1252` bytes the declaration names decode, the
/// Latin-1-supplement name surviving the round-trip to the `UTF-8` form.
#[test]
fn reads_windows_1252() {
    let name = "Caffè Società Perché S.r.l.";
    let original = domestic_invoice(&seller_with(name));
    let reencoded = reencode_as(
        &original.to_xml_vec().expect("the invoice serialises"),
        "windows-1252",
    );
    assert!(
        reencoded.contains(&0xE8),
        "the re-encoded document carries the windows-1252 è byte"
    );
    support::validate(&support::fpr12_schema(), &reencoded)
        .expect("the windows-1252 document validates against the schema");
    let parsed =
        FatturaElettronica::from_xml_slice(&reencoded).expect("the windows-1252 invoice reads");
    assert_eq!(parsed, original, "the windows-1252 invoice round-trips");
}

/// A declaration naming an `encoding` the WHATWG labels do not resolve is
/// rejected, naming it.
#[test]
fn rejects_an_unresolvable_encoding() {
    let original = domestic_invoice(&seller_with("ALPHA SRL"));
    let text = std::str::from_utf8(&original.to_xml_vec().expect("the invoice serialises"))
        .expect("the invoice serialises as UTF-8")
        .replace(r#"encoding="UTF-8""#, r#"encoding="not-an-encoding""#);
    let parsed = FatturaElettronica::from_xml_slice(text.as_bytes());
    assert!(
        matches!(
            parsed,
            Err(Error::UnsupportedEncoding(declared)) if declared == "not-an-encoding"
        ),
        "the reader names the unresolvable encoding"
    );
}

/// Encodes `text` as `UTF-16`, `big_endian` or little, and prepends the
/// matching byte order mark.
fn utf16_with_bom(text: &str, big_endian: bool) -> Vec<u8> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let bom = if big_endian { b"\xFE\xFF" } else { b"\xFF\xFE" };
    let mut bytes = Vec::with_capacity(bom.len() + units.len() * 2);
    bytes.extend_from_slice(bom);
    for unit in units {
        if big_endian {
            bytes.extend_from_slice(&unit.to_be_bytes());
        } else {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
    }
    bytes
}

/// A BOM-less `UTF-16` declaration reads its `ASCII` bytes as `UTF-8`, the
/// `WHATWG` output encoding the family reduces to.
#[test]
fn reads_utf16_declared_as_ascii() {
    let original = domestic_invoice(&seller_with("ALPHA SRL"));
    let text = std::str::from_utf8(&original.to_xml_vec().expect("the invoice serialises"))
        .expect("the invoice serialises as UTF-8")
        .replace(r#"encoding="UTF-8""#, r#"encoding="UTF-16""#);
    let parsed = FatturaElettronica::from_xml_slice(text.as_bytes())
        .expect("a UTF-16-declared ASCII document reads");
    assert_eq!(
        parsed, original,
        "the UTF-16-declared ASCII document round-trips"
    );
}

/// A `UTF-16LE` document with a byte order mark and non-`ASCII` text reads.
#[test]
fn reads_utf16le_with_bom() {
    let name = "Caffè Società Perché S.r.l.";
    let original = domestic_invoice(&seller_with(name));
    let text = std::str::from_utf8(&original.to_xml_vec().expect("the invoice serialises"))
        .expect("the invoice serialises as UTF-8")
        .replace(r#"encoding="UTF-8""#, r#"encoding="UTF-16LE""#);
    let bytes = utf16_with_bom(&text, false);
    let parsed =
        FatturaElettronica::from_xml_slice(&bytes).expect("the UTF-16LE BOM document reads");
    assert_eq!(parsed, original, "the UTF-16LE BOM document round-trips");
}

/// A `UTF-16BE` document with a byte order mark and non-`ASCII` text reads.
#[test]
fn reads_utf16be_with_bom() {
    let name = "Caffè Società Perché S.r.l.";
    let original = domestic_invoice(&seller_with(name));
    let text = std::str::from_utf8(&original.to_xml_vec().expect("the invoice serialises"))
        .expect("the invoice serialises as UTF-8")
        .replace(r#"encoding="UTF-8""#, r#"encoding="UTF-16BE""#);
    let bytes = utf16_with_bom(&text, true);
    let parsed =
        FatturaElettronica::from_xml_slice(&bytes).expect("the UTF-16BE BOM document reads");
    assert_eq!(parsed, original, "the UTF-16BE BOM document round-trips");
}

/// A `UTF-8` document with a byte order mark reads and round-trips.
#[test]
fn reads_utf8_with_bom() {
    let original = domestic_invoice(&seller_with("ALPHA SRL"));
    let utf8 = original.to_xml_vec().expect("the invoice serialises");
    let mut bytes = Vec::with_capacity(utf8.len() + 3);
    bytes.extend_from_slice(b"\xEF\xBB\xBF");
    bytes.extend_from_slice(&utf8);
    let parsed = FatturaElettronica::from_xml_slice(&bytes).expect("the UTF-8 BOM document reads");
    assert_eq!(parsed, original, "the UTF-8 BOM document round-trips");
}
