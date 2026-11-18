//! Verification of `XAdES` enveloped signatures, on real documents and on
//! the `tests/fixtures/xades` invoices signed with the test PKI.
#![expect(
    clippy::expect_used,
    reason = "a test fails loudly on any unexpected fixture"
)]

use std::fs;

use der::DecodePem;
use fatturapa::signed::{
    ChainError, ReferenceTarget, SignatureDefect, SignatureElement, SignedError, VerifiedSignature,
    verify_xades,
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
        ("key-info-two-certificates", |error| {
            matches!(
                error,
                SignedError::MalformedSignature(SignatureDefect::Repeated(
                    SignatureElement::X509Certificate
                ))
            )
        }),
        ("issuer-serial-v2-not-der", |error| {
            matches!(
                error,
                SignedError::MalformedSignature(SignatureDefect::NotDer(
                    SignatureElement::IssuerSerialV2
                ))
            )
        }),
        (
            "critical-extension-signer",
            |error| matches!(error, SignedError::UnsupportedAlgorithm(what) if what == "the signer certificate"),
        ),
        (
            "ed25519-key-rsa-method",
            |error| matches!(error, SignedError::UnsupportedAlgorithm(uri) if uri.ends_with("#rsa-sha256")),
        ),
        ("no-signing-time", |error| {
            matches!(error, SignedError::MissingSigningTime)
        }),
        ("no-signed-signature-properties", |error| {
            matches!(error, SignedError::MissingSigningTime)
        }),
    ];
    for (name, expected) in cases {
        match verify_fixture(name) {
            Err(error) if expected(&error) => {}
            outcome => panic!("{name}: {outcome:?}"),
        }
    }
}

/// A signature without `xades:SigningTime` is checked at the time the
/// caller supplies, which the verified signature reports.
#[test]
fn a_supplied_time_stands_in_for_a_missing_signing_time() {
    let anchors = [cert("ca"), cert("intermediate")];
    for name in ["no-signing-time", "no-signed-signature-properties"] {
        let input = read(&format!("tests/fixtures/xades/{name}.xml"));
        let outcome = verify_xades(&input, &anchors, Some(fixture_signing_time()));
        assert!(
            matches!(&outcome, Ok(verified) if verified.signing_time() == &fixture_signing_time()),
            "{name}: {outcome:?}"
        );
    }
}

/// The digest error names the reference whose content changed.
#[test]
fn a_digest_error_names_its_reference() {
    for (name, message) in [
        (
            "invoice-changed",
            "the digest of the document does not match the signed digest",
        ),
        (
            "properties-changed",
            "the digest of #SignedProperties1 does not match the signed digest",
        ),
    ] {
        let error = verify_fixture(name).expect_err("the content changed after signing");
        assert_eq!(error.to_string(), message, "{name}");
    }
}

/// The `tsl-rsa-sha512` fixture with the first `from` replaced by `to`.
/// Its document reference leaves the signature out, and its references do
/// not cover `ds:KeyInfo` or `ds:SignedInfo`, so changes there reach the
/// checks after the digests.
fn changed_fixture(name: &str, from: &str, to: &str) -> Vec<u8> {
    let input = String::from_utf8(read(&format!("tests/fixtures/xades/{name}.xml")))
        .expect("the fixture is UTF-8");
    assert!(input.contains(from), "{name} carries {from}");
    input.replacen(from, to, 1).into_bytes()
}

/// Each structural defect of the unsigned parts of a signature fails with
/// the defect, and its message names the element.
#[test]
fn a_defect_outside_the_signed_content_is_reported() {
    let certificates_start = "<ds:X509Data><ds:X509Certificate>";
    let cases = [
        (
            changed_fixture(
                "tsl-rsa-sha512",
                certificates_start,
                "<ds:X509Data><ds:X509Certificate>AAAA</ds:X509Certificate></ds:X509Data><ds:X509Data><ds:X509Certificate>",
            ),
            "malformed XML signature: the X509Certificate element is not DER",
        ),
        (
            changed_fixture(
                "tsl-rsa-sha512",
                "<ds:X509Data>",
                "<ds:X509Data><ds:X509Certificate>@@</ds:X509Certificate>",
            ),
            "malformed XML signature: the X509Certificate element is not base64",
        ),
        (
            changed_fixture(
                "tsl-rsa-sha512",
                "</ds:Signature>",
                r#"<ds:Object><xades:QualifyingProperties xmlns:xades="http://uri.etsi.org/01903/v1.3.2#"><xades:SignedProperties/></xades:QualifyingProperties></ds:Object></ds:Signature>"#,
            ),
            "malformed XML signature: more than one SignedProperties element",
        ),
    ];
    for (input, message) in cases {
        let error =
            verify_xades(&input, &[cert("ca")], None).expect_err("the signature is malformed");
        assert_eq!(error.to_string(), message);
    }
    let without_certificates = String::from_utf8(read("tests/fixtures/xades/tsl-rsa-sha512.xml"))
        .expect("the fixture is UTF-8");
    let start = without_certificates
        .find("<ds:X509Data>")
        .expect("the fixture has X509Data");
    let end = without_certificates
        .find("</ds:X509Data>")
        .expect("the fixture closes X509Data");
    let mut input = without_certificates;
    input.replace_range(start..end + "</ds:X509Data>".len(), "");
    let outcome = verify_xades(input.as_bytes(), &[cert("ca")], None);
    assert!(
        matches!(
            outcome,
            Err(SignedError::MalformedSignature(SignatureDefect::Missing(
                SignatureElement::X509Certificate
            )))
        ),
        "{outcome:?}"
    );
}

/// An algorithm of `ds:SignedInfo` the verifier does not support, or a
/// signature method that does not fit the signer key, is refused.
#[test]
fn signed_info_algorithms_the_verifier_does_not_support_are_refused() {
    let rsa_sha256 = "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256";
    let cases = [
        (
            changed_fixture(
                "tsl-rsa-sha512",
                r#"<ds:CanonicalizationMethod Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"/>"#,
                r#"<ds:CanonicalizationMethod Algorithm="urn:unknown"/>"#,
            ),
            "urn:unknown",
        ),
        (
            changed_fixture(
                "inclusive-ec",
                "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha256",
                rsa_sha256,
            ),
            rsa_sha256,
        ),
    ];
    for (input, algorithm) in cases {
        let outcome = verify_xades(&input, &[cert("ca")], None);
        assert!(
            matches!(&outcome, Err(SignedError::UnsupportedAlgorithm(uri)) if uri == algorithm),
            "{algorithm}: {outcome:?}"
        );
    }
}

/// Every `ds:Signature` child of the document element must verify, not only
/// the first.
#[test]
fn a_second_signature_that_does_not_verify_fails_the_document() {
    let input = changed_fixture(
        "foreign-signature-element",
        "</p:FatturaElettronica>",
        r#"<ds:Signature xmlns:ds="http://www.w3.org/2000/09/xmldsig#"/></p:FatturaElettronica>"#,
    );
    let outcome = verify_xades(&input, &[cert("intermediate")], None);
    assert!(
        matches!(
            outcome,
            Err(SignedError::MalformedSignature(SignatureDefect::Missing(
                SignatureElement::SignedInfo
            )))
        ),
        "{outcome:?}"
    );
}

#[test]
fn input_that_is_not_xml_is_refused() {
    let outcome = verify_xades(b"<r><unclosed></r>", &[], None);
    assert!(
        matches!(outcome, Err(SignedError::NotXml(_))),
        "{outcome:?}"
    );
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
        (
            enveloped,
            format!(
                r#"<ds:Transform Algorithm="{exclusive}"/><ds:Transform Algorithm="{enveloped}"/>"#
            ),
        ),
    ];
    let subtracted = [
        "/descendant::Signature",
        "/descendant::xml:Signature",
        "/descendant::z:Signature",
    ]
    .map(|xpath| {
        (
            FILTER2,
            format!(
                r#"<ds:Transform Algorithm="{FILTER2}"><XPath xmlns="{FILTER2}" Filter="subtract">{xpath}</XPath></ds:Transform>"#
            ),
        )
    });
    for (algorithm, transforms) in cases.into_iter().chain(subtracted) {
        let outcome = verify_xades(&reference_document("", &transforms), &[], None);
        assert!(
            matches!(&outcome, Err(SignedError::UnsupportedTransform(uri)) if uri == algorithm),
            "{transforms}: {outcome:?}"
        );
    }
}

/// A reference the verifier cannot read fails before any digest.
#[test]
fn a_reference_that_cannot_be_read_is_reported() {
    let digest_value = String::from_utf8(reference_document("", ""))
        .expect("the document is UTF-8")
        .replace("AAAA", "@@");
    let cases = [
        (
            reference_document("", "</ds:Transforms><ds:Transforms>"),
            "malformed XML signature: more than one Transforms element",
        ),
        (
            digest_value.into_bytes(),
            "malformed XML signature: the DigestValue element is not base64",
        ),
        (
            br#"<r><ds:Signature xmlns:ds="http://www.w3.org/2000/09/xmldsig#"><ds:SignedInfo/></ds:Signature></r>"#.to_vec(),
            "malformed XML signature: no Reference element",
        ),
        (
            reference_document("#missing", ""),
            "no element has the referenced id missing",
        ),
    ];
    for (input, message) in cases {
        let error = verify_xades(&input, &[], None).expect_err("the reference cannot be read");
        assert_eq!(error.to_string(), message);
    }
}
