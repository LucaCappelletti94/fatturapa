//! Verification of `XAdES` enveloped signatures, on real documents and on
//! the `tests/fixtures/xades` invoices signed with the test PKI.
#![expect(
    clippy::expect_used,
    reason = "a test fails loudly on any unexpected fixture"
)]

use std::fs;

use der::DecodePem;
use fatturapa::signed::{
    ChainError, ReferenceTarget, SignedError, VerifiedSignature, verify_xades,
};
use fatturapa::{Document, FatturaElettronica};
use jiff::{SignedDuration, Timestamp};
use x509_cert::Certificate;

/// The bytes of the repository file at `path`.
fn read(path: &str) -> Vec<u8> {
    fs::read(path).expect("the fixture is readable")
}

/// The test PKI certificate `name`.
fn cert(name: &str) -> Certificate {
    Certificate::from_pem(read(&format!("tests/fixtures/{name}.pem")).as_slice())
        .expect("the fixture is a PEM certificate")
}

/// Verifies the signed invoice fixture `name` under the test root.
fn verify_fixture(name: &str) -> Result<VerifiedSignature, SignedError> {
    verify_xades(
        &read(&format!("tests/fixtures/xades/{name}.xml")),
        &[cert("ca")],
        None,
    )
}

/// The `SigningTime` of every fixture, one hour after the start of the
/// intermediate's validity.
fn fixture_signing_time() -> Timestamp {
    let not_before = cert("intermediate")
        .tbs_certificate()
        .validity()
        .not_before
        .to_system_time();
    Timestamp::try_from(not_before)
        .and_then(|start| start.checked_add(SignedDuration::from_hours(1)))
        .expect("the validity start is a representable instant")
}

/// Every signature shape verifies to its signer, its chain to the test root
/// and its `SigningTime`, and the signed bytes read back as the invoice.
#[test]
fn every_signature_shape_verifies() {
    for (name, signer) in [
        ("tsl-rsa-sha512", "rsa-signer"),
        ("inclusive-ec", "ec-signer"),
        ("comments-ec384", "ec384-signer"),
        ("inclusive-rsa-sha384", "rsa-signer"),
    ] {
        let verified = verify_fixture(name).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(verified.signer(), &cert(signer), "{name}");
        assert_eq!(
            verified.chain(),
            [cert(signer), cert("intermediate"), cert("ca")],
            "{name}"
        );
        assert_eq!(verified.signing_time(), &fixture_signing_time(), "{name}");
        verified
            .document::<FatturaElettronica>()
            .expect("the signed bytes are the invoice");
    }
}

/// The SDI message shape protects its single `KeyInfo` certificate by a
/// reference, and the chain needs the intermediate as the anchor.
#[test]
fn a_signed_key_info_protects_the_signer() {
    let input = read("tests/fixtures/xades/sdi-rsa.xml");
    let verified =
        verify_xades(&input, &[cert("intermediate")], None).expect("the signature verifies");
    assert_eq!(verified.signer(), &cert("rsa-signer"));
    assert_eq!(verified.chain(), [cert("rsa-signer"), cert("intermediate")]);
    FatturaElettronica::from_xml_slice(verified.content())
        .expect("the signed bytes are the invoice");
}

/// The `xmldsig-filter2` subtraction removes only the `XMLDSig` signatures,
/// so an element named `Signature` in another namespace stays signed.
#[test]
fn a_foreign_signature_element_stays_signed() {
    let input = read("tests/fixtures/xades/foreign-signature-element.xml");
    verify_xades(&input, &[cert("intermediate")], None).expect("the signature verifies");
}

/// Whether an error is the one a negative case expects.
type Expected = fn(&SignedError) -> bool;

/// Each negative fixture fails with the error naming its one defect.
#[test]
fn every_negative_case_fails_with_its_error() {
    let cases: &[(&str, Expected)] = &[
        ("invoice-changed", |error| {
            matches!(
                error,
                SignedError::ReferenceDigestMismatch(ReferenceTarget::Document)
            )
        }),
        (
            "properties-changed",
            |error| matches!(error, SignedError::ReferenceDigestMismatch(ReferenceTarget::Element(id)) if id == "SignedProperties1"),
        ),
        ("signed-info-changed", |error| {
            matches!(error, SignedError::SignatureInvalid)
        }),
        ("no-document-reference", |error| {
            matches!(error, SignedError::DocumentNotCovered)
        }),
        ("no-properties-reference", |error| {
            matches!(error, SignedError::MissingSignedProperties)
        }),
        ("unprotected-signer", |error| {
            matches!(error, SignedError::SignerCertificateNotProtected)
        }),
        ("other-certificate", |error| {
            matches!(error, SignedError::SigningCertificateMismatch)
        }),
        (
            "ambiguous-id",
            |error| matches!(error, SignedError::AmbiguousId(id) if id == "SignedProperties1"),
        ),
        (
            "external-reference",
            |error| matches!(error, SignedError::UnsupportedReference(Some(uri)) if uri == "http://example.com/invoice.xml"),
        ),
        (
            "xslt-transform",
            |error| matches!(error, SignedError::UnsupportedTransform(uri) if uri == "http://www.w3.org/TR/1999/REC-xslt-19991116"),
        ),
        (
            "rsa-sha1",
            |error| matches!(error, SignedError::UnsupportedAlgorithm(uri) if uri == "http://www.w3.org/2000/09/xmldsig#rsa-sha1"),
        ),
        (
            "sha1-digest",
            |error| matches!(error, SignedError::UnsupportedAlgorithm(uri) if uri == "http://www.w3.org/2000/09/xmldsig#sha1"),
        ),
        ("doctype", |error| {
            matches!(error, SignedError::DoctypeRefused)
        }),
        ("expired-signer", |error| {
            matches!(error, SignedError::Chain(ChainError::Expired))
        }),
        ("issuer-serial-mismatch", |error| {
            matches!(error, SignedError::SigningCertificateMismatch)
        }),
    ];
    for (name, expected) in cases {
        match verify_fixture(name) {
            Err(error) if expected(&error) => {}
            outcome => panic!("{name}: {outcome:?}"),
        }
    }
}

#[test]
fn an_unrelated_anchor_does_not_end_the_chain() {
    let input = read("tests/fixtures/xades/tsl-rsa-sha512.xml");
    let outcome = verify_xades(&input, &[cert("other-root")], None);
    assert!(
        matches!(
            outcome,
            Err(SignedError::Chain(ChainError::IssuerNotInStore))
        ),
        "{outcome:?}"
    );
}

/// The `AgID` list verifies under the qualified CA anchors it lists itself,
/// its signer chaining to `Ministero della Difesa - CA di Firma Digitale`.
#[cfg(feature = "trust-list")]
#[test]
fn the_agid_list_verifies_under_its_own_anchors() {
    let tsl = read("tests/fixtures/TSL-IT.xml");
    let anchors = fatturapa::trust_list::trust_anchors(&tsl).expect("the list parses");
    let verified = verify_xades(&tsl, &anchors, None).expect("the list signature verifies");
    assert_eq!(verified.content(), tsl.as_slice());
    assert_eq!(verified.chain().len(), 2);
    assert_eq!(verified.chain().first(), Some(verified.signer()));
    assert_eq!(verified.signing_time().to_string(), "2026-09-24T08:22:20Z");
    let anchor = verified
        .chain()
        .last()
        .expect("the chain ends at an anchor");
    assert!(
        anchor
            .tbs_certificate()
            .subject()
            .to_string()
            .contains("Ministero della Difesa"),
        "{}",
        anchor.tbs_certificate().subject()
    );
}

/// The SDI messages verify up to the chain, whose `Agenzia delle Entrate`
/// issuer no anchor names.
#[test]
fn sdi_messages_verify_up_to_their_unknown_issuer() {
    for file in [
        "tests/corpus/slamdunk/ok_ITHVQWPH73P42H501Y_X0024.xml",
        "tests/fixtures/reader_xpath/default_ns.xml",
    ] {
        let outcome = verify_xades(&read(file), &[], None);
        assert!(
            matches!(
                outcome,
                Err(SignedError::Chain(ChainError::IssuerNotInStore))
            ),
            "{file}: {outcome:?}"
        );
    }
}

/// The specification samples digest correctly but are signed with a
/// 1024-bit `RSA` key, below the 2048-bit minimum of the accepted
/// algorithms, which the signature check reports as invalid.
#[test]
fn the_specification_samples_carry_a_key_below_the_minimum() {
    for kind in ["AT", "DT", "MC", "NE", "NS", "RC", "SE"] {
        let file = format!("spec/samples/IT01234567890_11111_{kind}_001.xml");
        let outcome = verify_xades(&read(&file), &[], None);
        assert!(
            matches!(outcome, Err(SignedError::SignatureInvalid)),
            "{file}: {outcome:?}"
        );
    }
}

/// The `prefixed_ns` fixture is `default_ns` with the `xmldsig-filter2`
/// `XPath` of its `SignedInfo` rewritten under a prefix, so the signature no
/// longer verifies.
#[test]
fn a_signed_info_that_changed_after_signing_is_invalid() {
    let outcome = verify_xades(
        &read("tests/fixtures/reader_xpath/prefixed_ns.xml"),
        &[],
        None,
    );
    assert!(
        matches!(outcome, Err(SignedError::SignatureInvalid)),
        "{outcome:?}"
    );
}

/// The `gobl-sdi` invoices carry a placeholder signature over other content.
#[test]
fn a_placeholder_signature_does_not_cover_the_invoice() {
    let outcome = verify_xades(
        &read("tests/corpus/gobl-sdi/fatturapa.gobl/invoice-simple.xml"),
        &[],
        None,
    );
    assert!(
        matches!(
            outcome,
            Err(SignedError::ReferenceDigestMismatch(
                ReferenceTarget::Document
            ))
        ),
        "{outcome:?}"
    );
}

/// An unsigned invoice has no signature to verify.
#[test]
fn an_unsigned_invoice_is_not_signed() {
    let outcome = verify_xades(&read("spec/samples/IT01234567890_FPR01.xml"), &[], None);
    assert!(
        matches!(outcome, Err(SignedError::NotSigned)),
        "{outcome:?}"
    );
}

/// The `xmldsig-filter2` transform.
const FILTER2: &str = "http://www.w3.org/2002/06/xmldsig-filter2";

/// A document whose one signature has one reference with `uri` and the
/// `transforms` markup. References are checked before anything else of the
/// signature, so its other parts are left out.
fn reference_document(uri: &str, transforms: &str) -> Vec<u8> {
    format!(
        r#"<r><ds:Signature xmlns:ds="http://www.w3.org/2000/09/xmldsig#"><ds:SignedInfo><ds:Reference URI="{uri}"><ds:Transforms>{transforms}</ds:Transforms><ds:DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/><ds:DigestValue>AAAA</ds:DigestValue></ds:Reference></ds:SignedInfo></ds:Signature></r>"#
    )
    .into_bytes()
}

#[test]
fn an_xpointer_fragment_is_unsupported() {
    let outcome = verify_xades(&reference_document("#xpointer(/)", ""), &[], None);
    assert!(
        matches!(&outcome, Err(SignedError::UnsupportedReference(Some(uri))) if uri == "#xpointer(/)"),
        "{outcome:?}"
    );
}

/// Every transform that would remove more than the signatures, or that
/// carries parameters its algorithm does not take, is refused.
#[test]
fn transforms_other_than_the_supported_shapes_are_refused() {
    let enveloped = "http://www.w3.org/2000/09/xmldsig#enveloped-signature";
    let inclusive = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315";
    let exclusive = "http://www.w3.org/2001/10/xml-exc-c14n#";
    let cases = [
        (
            FILTER2,
            format!(
                r#"<ds:Transform Algorithm="{FILTER2}"><XPath xmlns="{FILTER2}" Filter="intersect">/descendant::ds:Signature</XPath></ds:Transform>"#
            ),
        ),
        (
            FILTER2,
            format!(
                r#"<ds:Transform Algorithm="{FILTER2}"><XPath xmlns="{FILTER2}" xmlns:x="urn:other" Filter="subtract">/descendant::x:Signature</XPath></ds:Transform>"#
            ),
        ),
        (
            enveloped,
            format!(
                r#"<ds:Transform Algorithm="{enveloped}"><ds:XPath>/</ds:XPath></ds:Transform>"#
            ),
        ),
        (
            inclusive,
            format!(
                r#"<ds:Transform Algorithm="{inclusive}"><ec:InclusiveNamespaces xmlns:ec="{exclusive}" PrefixList="ds"/></ds:Transform>"#
            ),
        ),
        (
            exclusive,
            format!(
                r#"<ds:Transform Algorithm="{exclusive}"><o:InclusiveNamespaces xmlns:o="urn:other" PrefixList="ds"/></ds:Transform>"#
            ),
        ),
        (
            exclusive,
            format!(
                r#"<ds:Transform Algorithm="{exclusive}"><ec:Other xmlns:ec="{exclusive}"/></ds:Transform>"#
            ),
        ),
    ];
    for (algorithm, transforms) in cases {
        let outcome = verify_xades(&reference_document("", &transforms), &[], None);
        assert!(
            matches!(&outcome, Err(SignedError::UnsupportedTransform(uri)) if uri == algorithm),
            "{transforms}: {outcome:?}"
        );
    }
}
