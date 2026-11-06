//! Shared helpers for the integration tests: offline XSD validation with
//! `xmllint` and the schema and sample paths.

#![expect(
    dead_code,
    reason = "the shared helpers are used by different test binaries"
)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
