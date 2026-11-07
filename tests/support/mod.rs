//! Shared helpers for the integration tests: offline XSD validation with
//! `xmllint`, the schema and sample paths and the parties of the
//! fixture invoices.

#![expect(
    dead_code,
    reason = "the shared helpers are used by different test binaries"
)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use fatturapa::ids::{EuCountry, Provincia, RegimeFiscale};
use fatturapa::party::{Address, Business, Buyer, Company, Seller};
use iso_country::Country;

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
