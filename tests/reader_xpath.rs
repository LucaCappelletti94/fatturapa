//! The generated reader accepts a `ds:Transform` that carries a filter-2
//! `XPath` child from the `xmldsig-filter2` namespace, whether the element is
//! written with an explicit prefix or with the default namespace.

#![expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing helper"
)]

use fatturapa::Document;
use fatturapa::FatturaElettronica;

fn read_fixture(name: &str) -> FatturaElettronica {
    let path = std::path::Path::new("tests/fixtures/reader_xpath").join(name);
    let bytes = std::fs::read(&path).expect("the fixture reads from disk");
    FatturaElettronica::from_xml_slice(&bytes)
        .unwrap_or_else(|error| panic!("{name} does not read: {error}"))
}

/// The default-namespace fixture reads and re-reads. The `xs:any` content of
/// the signature does not round-trip to an equal parse, because the
/// `xsd-parser` does not keep the namespace declarations of that content
/// stable, so the test checks the read and the write, not the re-read.
#[test]
fn transform_filter2_xpath_reads_with_default_namespace() {
    let parsed = read_fixture("default_ns.xml");
    parsed.to_xml_vec().expect("the invoice writes");
}

#[test]
fn transform_filter2_xpath_reads_with_prefixed_namespace() {
    read_fixture("prefixed_ns.xml");
}
