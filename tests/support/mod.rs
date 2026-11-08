//! Shared helpers for the integration tests: offline XSD validation with
//! `xmllint`, the schema and sample paths and the parties of the
//! fixture invoices.

#![expect(
    dead_code,
    reason = "the shared helpers are used by different test binaries"
)]

use std::fmt::Debug;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use fatturapa::generated::{fpr12, fsm10};
use fatturapa::ids::{EuCountry, Provincia, RegimeFiscale};
use fatturapa::party::{Address, Business, Buyer, Company, Seller};
use fatturapa::validate::Violation;
use fatturapa::{Document, FatturaElettronica, FatturaElettronicaSemplificata};
use iso_country::Country;
use xsd_parser_types::xml::{AnyElement, Element, Value};

/// The in-scope namespace declarations of a parsed `xs:any` element, on
/// it and on every element inside it, are the only part that varies across
/// a read-modify-write cycle, when the source prefix differs from the one
/// the writer picks. The normalizer clears them and keeps the name, the
/// attributes and the content.
fn normalize_element(element: Element<'static>) -> Element<'static> {
    let Element { name, values, .. } = element;
    Element {
        name,
        values: values
            .into_iter()
            .map(|value| match value {
                Value::Element(nested) => Value::Element(normalize_element(nested)),
                other => other,
            })
            .collect(),
        ..Element::default()
    }
}

/// Normalise a parsed `xs:any` element, clearing the in-scope namespaces
/// at every level and keeping the name, the attributes and the content.
fn normalize_any(any: AnyElement) -> AnyElement {
    normalize_element(any)
}

/// Generate the `xs:any` normalizer for a module's XML-DSig `Signature`.
macro_rules! normalize_signature_impl {
    ($module:ident, $name:ident) => {
        /// Return `signature` with every `xs:any` element normalised, so
        /// that the signature of the first read and of the re-read of the
        /// same bytes compare equal.
        pub(crate) fn $name(mut signature: $module::Signature) -> $module::Signature {
            let signed_info = &mut signature.content.signed_info.content;
            for any in signed_info.canonicalization_method.content.any.iter_mut() {
                *any = normalize_any(std::mem::take(any));
            }
            for any in signed_info.signature_method.content.any.iter_mut() {
                *any = normalize_any(std::mem::take(any));
            }
            for reference in signed_info.reference.iter_mut() {
                if let Some(transforms) = &mut reference.content.transforms {
                    for transform in transforms.content.transform.iter_mut() {
                        for content in transform.content.iter_mut() {
                            if let $module::TransformTypeContent::Any(any) = content {
                                *any = normalize_any(std::mem::take(any));
                            }
                        }
                    }
                }
                for any in reference.content.digest_method.content.any.iter_mut() {
                    *any = normalize_any(std::mem::take(any));
                }
            }
            if let Some(key_info) = &mut signature.content.key_info {
                for content in key_info.content.iter_mut() {
                    match content {
                        $module::KeyInfoTypeContent::Any(any) => {
                            *any = normalize_any(std::mem::take(any));
                        }
                        $module::KeyInfoTypeContent::KeyValue(key_value) => {
                            if let $module::KeyValueTypeContent::Any(any) = &mut key_value.content {
                                *any = normalize_any(std::mem::take(any));
                            }
                        }
                        $module::KeyInfoTypeContent::X509Data(x509_data) => {
                            for entry in x509_data.content.iter_mut() {
                                if let $module::X509DataContent19TypeContent::Any(any) =
                                    &mut entry.content_19.content
                                {
                                    *any = normalize_any(std::mem::take(any));
                                }
                            }
                        }
                        $module::KeyInfoTypeContent::PgpData(pgp_data) => {
                            if let $module::PgpDataTypeContent::Content23(content) =
                                &mut pgp_data.content
                            {
                                for any in content.content.any.iter_mut() {
                                    *any = normalize_any(std::mem::take(any));
                                }
                            }
                            if let $module::PgpDataTypeContent::Content25(content) =
                                &mut pgp_data.content
                            {
                                for any in content.content.any.iter_mut() {
                                    *any = normalize_any(std::mem::take(any));
                                }
                            }
                        }
                        $module::KeyInfoTypeContent::SpkiData(spki_data) => {
                            for content in spki_data.content.iter_mut() {
                                if let Some(any) = &mut content.any {
                                    *any = normalize_any(std::mem::take(any));
                                }
                            }
                        }
                        $module::KeyInfoTypeContent::KeyName(_)
                        | $module::KeyInfoTypeContent::RetrievalMethod(_)
                        | $module::KeyInfoTypeContent::MgmtData(_) => {}
                    }
                }
            }
            for object in signature.content.object.iter_mut() {
                for content in object.content.iter_mut() {
                    content.any = normalize_any(std::mem::take(&mut content.any));
                }
            }
            signature
        }
    };
}

normalize_signature_impl!(fpr12, normalize_fpr12_signature);
normalize_signature_impl!(fsm10, normalize_fsm10_signature);

/// The XML catalog that maps the W3C XML-DSig address onto the local copy.
pub(crate) fn catalog() -> PathBuf {
    Path::new("spec/catalog.xml").to_path_buf()
}

/// The ordinary invoice schema `Schema_VFPR12_v1.2.3.xsd`.
pub(crate) fn fpr12_schema() -> PathBuf {
    Path::new("spec/Schema_VFPR12_v1.2.3.xsd").to_path_buf()
}

/// The simplified invoice schema `Schema_VFSM10v_1.0.2.xsd`.
pub(crate) fn fsm10_schema() -> PathBuf {
    Path::new("spec/Schema_VFSM10v_1.0.2.xsd").to_path_buf()
}

/// The SDI message schema `MessaggiTypes_v1.1.xsd`.
pub(crate) fn sdi_schema() -> PathBuf {
    Path::new("spec/MessaggiTypes_v1.1.xsd").to_path_buf()
}

/// The path of the official sample with file name `file`.
pub(crate) fn sample(file: &str) -> PathBuf {
    Path::new("spec/samples").join(file)
}

/// The bytes of the official sample with file name `file`.
pub(crate) fn sample_bytes(file: &str) -> Vec<u8> {
    std::fs::read(sample(file)).expect("the sample is checked in")
}

/// The path of the test fixture with file name `file`, in `tests/fixtures`.
pub(crate) fn fixture(file: &str) -> PathBuf {
    Path::new("tests/fixtures").join(file)
}

/// The bytes of the test fixture with file name `file`, in `tests/fixtures`.
pub(crate) fn fixture_bytes(file: &str) -> Vec<u8> {
    std::fs::read(fixture(file)).expect("the fixture is checked in")
}

/// Asserts that `violations` carries the SDI error `code`.
pub(crate) fn assert_has(violations: &[Violation], code: &str) {
    assert!(
        violations.iter().any(|v| v.code == code),
        "expected {code}, got {violations:?}"
    );
}

/// Reads the invoice fixture `file` from `tests/fixtures/invoices`, validates
/// the checked-in bytes against `schema`, reads them, writes them back,
/// validates the written bytes and compares the re-read with the first read.
fn invoice_round_trips<T>(file: &str, schema: &Path) -> T
where
    T: Document + PartialEq + Debug,
{
    let bytes = fixture_bytes(&format!("invoices/{file}"));
    validate(schema, &bytes).expect("the fixture validates against the schema");
    let doc = T::from_xml_slice(&bytes).expect("the fixture reads");
    let written = doc.to_xml_vec().expect("the fixture serialises");
    validate(schema, &written).expect("the written fixture validates against the schema");
    let reread = T::from_xml_slice(&written).expect("the fixture re-reads");
    assert_eq!(reread, doc, "the fixture round-trips equal");
    doc
}

/// The round trip of an ordinary invoice fixture, against the FPR12 schema.
pub(crate) fn fixture_round_trips(file: &str) -> FatturaElettronica {
    invoice_round_trips(file, &fpr12_schema())
}

/// The round trip of a simplified invoice fixture, against the FSM10 schema.
pub(crate) fn fsm10_round_trips(file: &str) -> FatturaElettronicaSemplificata {
    invoice_round_trips(file, &fsm10_schema())
}

/// The Italian seller of the fixture invoices.
pub(crate) fn seller() -> Seller {
    Seller::denomination(
        "ALPHA SRL".parse().expect("a valid denomination"),
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

/// The domestic Italian buyer of the fixture invoices.
pub(crate) fn buyer() -> Buyer {
    Buyer::denomination(
        "BETA SRL".parse().expect("a valid denomination"),
        "04444444444".parse().expect("a valid partita IVA"),
        Address::italian(
            "VIA MILANO 2".parse().expect("a valid address"),
            "20100".parse().expect("a valid CAP"),
            "MILANO".parse().expect("a valid comune"),
            Provincia::Mi,
        ),
    )
}

/// The Italian buyer company of the self-invoice fixtures, whose
/// `partita IVA` transmits the document.
pub(crate) fn buyer_company() -> Company {
    Company::denomination(
        "BETA SRL".parse().expect("a valid denomination"),
        "04444444444".parse().expect("a valid partita IVA"),
        Address::italian(
            "VIA MILANO 2".parse().expect("a valid address"),
            "20100".parse().expect("a valid CAP"),
            "MILANO".parse().expect("a valid comune"),
            Provincia::Mi,
        ),
    )
}

/// The US business party of the art. 7-ter and self-invoice fixtures,
/// without an Italian VAT number, so its `IdFiscaleIVA` carries the
/// country of residence and the fiscal identifier assigned there.
pub(crate) fn business_us() -> Business {
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

/// The German business party of the art. 7-ter and self-invoice fixtures.
pub(crate) fn business_de() -> Business {
    Business::eu(
        EuCountry::De,
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

/// The French business party of the intracommunity self-invoice fixtures.
pub(crate) fn business_fr() -> Business {
    Business::eu(
        EuCountry::Fr,
        "FICTIF SAS".parse().expect("a valid denomination"),
        "FR93456789012".parse().expect("a valid identifier"),
        Address::foreign(
            "12 RUE DES LILAS".parse().expect("a valid address"),
            Some("75008".parse().expect("a valid postal code")),
            "PARIS".parse().expect("a valid comune"),
            Country::FR,
        )
        .expect("a valid foreign seat"),
    )
}

/// Validates `doc` against `schema` with `xmllint`, offline, using the
/// catalog in `spec/catalog.xml`.
///
/// # Errors
///
/// Fails when `xmllint` cannot be found or when the document does not
/// validate, with the `xmllint` output on failure.
pub(crate) fn validate(schema: &Path, doc: &[u8]) -> Result<(), String> {
    let mut child = Command::new("xmllint")
        .arg("--nonet")
        .arg("--noout")
        .arg("--schema")
        .arg(schema)
        .arg("-")
        .env("XML_CATALOG_FILES", catalog())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn xmllint from PATH: {error}"))?;
    {
        let mut stdin = child.stdin.take().expect("the stdin pipe is set to piped");
        stdin
            .write_all(doc)
            .and_then(|()| stdin.flush())
            .expect("the xmllint stdin pipe is open");
    }
    let output = child.wait_with_output().expect("xmllint was spawned");
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}
