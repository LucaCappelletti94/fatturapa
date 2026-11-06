//! The schema-aware round trip. The seed of the fuzz input builds an
//! arbitrary document of one of the eleven root types through the generator
//! the property tests share, which keeps every value inside the facets of
//! the schemas. The document is written, validated against its official
//! `XSD` with `xmllint` when the tool is on `PATH`, re-read and compared
//! with the original. The seven `SDI` message types that require a
//! `ds:Signature` skip the validation until the upstream `xsd-parser` fix
//! lands.

#![no_main]

#[path = "../../tests/generator/mod.rs"]
mod generator;

use core::cmp::PartialEq;
use core::fmt::Debug;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use fatturapa::Document;
use fatturapa::{
    AttestazioneTrasmissioneFattura, NotificaDecorrenzaTermini, NotificaEsito,
    NotificaEsitoCommittente, NotificaMancataConsegna, NotificaScarto, RicevutaConsegna,
    ScartoEsitoCommittente,
};
use libfuzzer_sys::fuzz_target;

/// The schemas, anchored to the crate root that contains this fuzz crate.
const FPR12_SCHEMA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../spec/Schema_VFPR12_v1.2.3.xsd"
);
const FSM10_SCHEMA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../spec/Schema_VFSM10v_1.0.2.xsd"
);
const SDI_SCHEMA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../spec/MessaggiTypes_v1.1.xsd"
);
const CATALOG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../spec/catalog.xml");

fuzz_target!(|seed: u64| {
    match seed % 11 {
        0 => round_trip(
            generator::invoice_from_seed(seed),
            FPR12_SCHEMA,
            "the invoice",
        ),
        1 => round_trip(
            generator::semplificata_from_seed(seed),
            FSM10_SCHEMA,
            "the simplified invoice",
        ),
        2 => round_trip(generator::mt_from_seed(seed), SDI_SCHEMA, "the MT message"),
        3 => without_validation::<NotificaEsitoCommittente>(
            generator::ec_from_seed(seed),
            "the EC message",
        ),
        4 => without_validation::<AttestazioneTrasmissioneFattura>(
            generator::at_from_seed(seed),
            "the AT message",
        ),
        5 => without_validation::<NotificaDecorrenzaTermini>(
            generator::dt_from_seed(seed),
            "the DT message",
        ),
        6 => without_validation::<NotificaMancataConsegna>(
            generator::mc_from_seed(seed),
            "the MC message",
        ),
        7 => without_validation::<NotificaEsito>(generator::ne_from_seed(seed), "the NE message"),
        8 => without_validation::<NotificaScarto>(generator::ns_from_seed(seed), "the NS message"),
        9 => {
            without_validation::<RicevutaConsegna>(generator::rc_from_seed(seed), "the RC message")
        }
        _ => without_validation::<ScartoEsitoCommittente>(
            generator::se_from_seed(seed),
            "the SE message",
        ),
    };
});

/// Writes the document, validates the bytes against `schema`, re-reads them
/// and compares the read with the original.
fn round_trip<D: Document + Debug + PartialEq>(doc: D, schema: &str, what: &str) {
    let bytes = doc
        .to_xml_vec()
        .unwrap_or_else(|error| panic!("{what} does not serialise: {error}"));
    validate(schema, &bytes, what);
    let reread = D::from_xml_slice(&bytes).unwrap_or_else(|error| {
        panic!(
            "{what} does not re-read: {error}\n{}",
            String::from_utf8_lossy(&bytes)
        )
    });
    assert_eq!(
        reread,
        doc,
        "{what} does not round-trip\n{}",
        String::from_utf8_lossy(&bytes)
    );
}

/// Writes the document, re-reads the bytes and compares the read with the
/// original, without the `XSD` validation that waits for the upstream
/// `xsd-parser` `ds:Signature` namespace fix.
fn without_validation<D: Document + Debug + PartialEq>(doc: D, what: &str) {
    let bytes = doc
        .to_xml_vec()
        .unwrap_or_else(|error| panic!("{what} does not serialise: {error}"));
    let reread = D::from_xml_slice(&bytes).unwrap_or_else(|error| {
        panic!(
            "{what} does not re-read: {error}\n{}",
            String::from_utf8_lossy(&bytes)
        )
    });
    assert_eq!(
        reread,
        doc,
        "{what} does not round-trip\n{}",
        String::from_utf8_lossy(&bytes)
    );
}

/// Validates `doc` against `schema` with `xmllint`, offline, using the
/// catalog next to the schemas. The ClusterFuzzLite runner image has no
/// `xmllint`, so the validation is skipped there and the write and re-read
/// checks alone run.
fn validate(schema: &str, doc: &[u8], what: &str) {
    let mut child = match Command::new("xmllint")
        .arg("--nonet")
        .arg("--noout")
        .arg("--schema")
        .arg(schema)
        .arg("-")
        .env("XML_CATALOG_FILES", CATALOG)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => panic!("failed to spawn xmllint to validate {what}: {error}"),
    };
    {
        let mut stdin = child.stdin.take().expect("the stdin pipe is piped");
        stdin
            .write_all(doc)
            .and_then(|()| stdin.flush())
            .expect("the stdin pipe is open");
    }
    let output = child.wait_with_output().expect("xmllint was spawned");
    if !output.status.success() {
        let reason = String::from_utf8_lossy(&output.stderr);
        let schema_name = Path::new(schema)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        panic!(
            "{what} does not validate against {schema_name}\n{reason}\ndocument:\n{}",
            String::from_utf8_lossy(doc)
        );
    }
}
