//! `XAdES-BES` enveloped signatures, read from the [`Tree`] of the signed
//! document.
//!
//! A `ds:Reference` is dereferenced, transformed and digested here. The
//! URIs read are `""`, the whole document, and `#id`, the one element
//! carrying the id. The transforms read are `enveloped-signature`, the
//! `xmldsig-filter2` subtraction of every `ds:Signature`, and the four
//! canonicalizations, which end the transform chain.
//!
//! A signature is accepted when every reference digests to its value, one
//! reference covers the whole document and one covers the
//! `xades:SignedProperties` of the signature, the signer certificate is
//! protected by a signing certificate property or a signed `ds:KeyInfo`, the
//! `ds:SignatureValue` verifies under the signer key, and the signer chains
//! to a trust anchor at the signing time.

use std::str::FromStr;

use base64::Engine;
use const_oid::db;
use der::Decode;
use der::asn1::ObjectIdentifier;
use digest::Digest;
use jiff::Timestamp;
use rustls_pki_types::CertificateDer;
use sha2::{Sha256, Sha384, Sha512};
use webpki::{EndEntityCert, Error as WebpkiError};
use x509_cert::Certificate;
use x509_cert::name::Name;

use super::c14n::{Canonicalization, Method, NodeId, Subset, Tree, TreeError, canonicalize};
use super::{
    IssuerSerial, ReferenceTarget, SignatureDefect, SignatureElement, SignedError,
    VerifiedSignature, ecdsa_curve, issuer_serial_matches, key_usage_permits_signing, names_match,
    signature_algorithm, verify_chain,
};

/// The `XMLDSig` namespace.
const DSIG: &[&str] = &["http://www.w3.org/2000/09/xmldsig#"];
/// The `XAdES` 1.3.2 and 1.4.1 namespaces.
const XADES: &[&str] = &[
    "http://uri.etsi.org/01903/v1.3.2#",
    "http://uri.etsi.org/01903/v1.4.1#",
];
/// The `enveloped-signature` transform.
const ENVELOPED: &str = "http://www.w3.org/2000/09/xmldsig#enveloped-signature";
/// The `xmldsig-filter2` transform and the namespace of its `XPath`.
const FILTER2: &str = "http://www.w3.org/2002/06/xmldsig-filter2";
/// Canonical `XML` 1.0.
const INCLUSIVE: &str = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315";
/// Canonical `XML` 1.0 with comments.
const INCLUSIVE_WITH_COMMENTS: &str =
    "http://www.w3.org/TR/2001/REC-xml-c14n-20010315#WithComments";
/// Exclusive `XML` canonicalization 1.0, also the namespace of its
/// `InclusiveNamespaces`.
const EXCLUSIVE: &str = "http://www.w3.org/2001/10/xml-exc-c14n#";
/// Exclusive `XML` canonicalization 1.0 with comments.
const EXCLUSIVE_WITH_COMMENTS: &str = "http://www.w3.org/2001/10/xml-exc-c14n#WithComments";

/// Verifies every `XAdES` enveloped signature of an `XML` document and
/// returns the document, the signer, the chain and the signing time of the
/// first.
///
/// The signatures read are the `ds:Signature` children of the document
/// element. `signing_time` is used for the validity checks of a signature
/// without an `xades:SigningTime`. The signer chain is built from the
/// `ds:KeyInfo` certificates and the trust anchors in `anchors`, whose own
/// validity is not checked.
///
/// # Errors
///
/// Fails with [`SignedError`] when the input is not an `XML` document,
/// carries a `DOCTYPE` or no signature, or when any signature does not
/// verify or has no chain to a trust anchor.
pub fn verify_xades(
    input: &[u8],
    anchors: &[Certificate],
    signing_time: Option<Timestamp>,
) -> Result<VerifiedSignature, SignedError> {
    let tree = Tree::parse(input).map_err(|error| match error {
        TreeError::Doctype => SignedError::DoctypeRefused,
        other => SignedError::NotXml(other.to_string()),
    })?;
    let mut outcomes = children_named(&tree, tree.root(), DSIG, "Signature")
        .map(|signature| verify_signature(&tree, signature, anchors, signing_time));
    let (signer, chain, signing_time) = outcomes.next().ok_or(SignedError::NotSigned)??;
    for outcome in outcomes {
        outcome?;
    }
    Ok(VerifiedSignature {
        content: input.to_vec(),
        signer,
        chain,
        signing_time,
    })
}

/// Verifies the `ds:Signature` `signature` and returns the signer
/// certificate, the chain and the signing time.
fn verify_signature(
    tree: &Tree,
    signature: NodeId,
    anchors: &[Certificate],
    signing_time: Option<Timestamp>,
) -> Result<(Certificate, Vec<Certificate>, Timestamp), SignedError> {
    let signed_info = signed_info(tree, signature)?;
    let mut document_covered = false;
    let mut covered = Vec::new();
    for reference in references(tree, signed_info)? {
        match check_reference(tree, signature, reference)? {
            Dereferenced::Document => document_covered = true,
            Dereferenced::Element(element) => covered.push(element),
        }
    }
    if !document_covered {
        return Err(SignedError::DocumentNotCovered);
    }
    let signed_properties = signed_properties(tree, signature)?;
    if !covered.contains(&signed_properties) {
        return Err(SignedError::MissingSignedProperties);
    }
    let key_info = required_child(tree, signature, DSIG, "KeyInfo", SignatureElement::KeyInfo)?;
    let certificates = key_info_certificates(tree, key_info)?;
    let signer = select_signer(
        tree,
        signed_properties,
        &certificates,
        covered.contains(&key_info),
    )?;
    let signing_time = match signing_time_property(tree, signed_properties)? {
        Some(time) => time,
        None => signing_time.ok_or(SignedError::MissingSigningTime)?,
    };
    let signer_der = CertificateDer::from(signer.der.as_slice());
    let end_entity = EndEntityCert::try_from(&signer_der)
        .map_err(|_| SignedError::UnsupportedAlgorithm("the signer certificate".to_owned()))?;
    check_signature_value(
        tree,
        signature,
        signed_info,
        &signer.certificate,
        &end_entity,
    )?;
    key_usage_permits_signing(&signer.certificate)?;
    let pool: Vec<Certificate> = certificates
        .iter()
        .map(|entry| entry.certificate.clone())
        .collect();
    let chain = verify_chain(
        &signer.certificate,
        &end_entity,
        signing_time,
        &pool,
        anchors,
    )?;
    Ok((signer.certificate.clone(), chain, signing_time))
}

/// A digest algorithm a `ds:DigestMethod` names.
#[derive(Clone, Copy, Debug)]
enum DigestAlgorithm {
    /// `SHA-256`.
    Sha256,
    /// `SHA-384`.
    Sha384,
    /// `SHA-512`.
    Sha512,
}

impl DigestAlgorithm {
    /// The algorithm the `uri` names.
    fn from_uri(uri: &str) -> Result<Self, SignedError> {
        match uri {
            "http://www.w3.org/2001/04/xmlenc#sha256" => Ok(Self::Sha256),
            "http://www.w3.org/2001/04/xmldsig-more#sha384" => Ok(Self::Sha384),
            "http://www.w3.org/2001/04/xmlenc#sha512" => Ok(Self::Sha512),
            other => Err(SignedError::UnsupportedAlgorithm(other.to_owned())),
        }
    }

    /// The digest of `data`.
    fn digest(self, data: &[u8]) -> Vec<u8> {
        match self {
            Self::Sha256 => Sha256::digest(data).to_vec(),
            Self::Sha384 => Sha384::digest(data).to_vec(),
            Self::Sha512 => Sha512::digest(data).to_vec(),
        }
    }
}

/// The canonicalization method and whether it keeps comments, for a `uri`
/// naming one of the four supported canonicalizations.
fn canonicalization_method(uri: &str) -> Option<(Method, bool)> {
    match uri {
        INCLUSIVE => Some((Method::Inclusive, false)),
        INCLUSIVE_WITH_COMMENTS => Some((Method::Inclusive, true)),
        EXCLUSIVE => Some((Method::Exclusive, false)),
        EXCLUSIVE_WITH_COMMENTS => Some((Method::Exclusive, true)),
        _ => None,
    }
}

/// The signature algorithm `OID` a `ds:SignatureMethod` `uri` names.
fn signature_method(uri: &str) -> Option<ObjectIdentifier> {
    match uri {
        "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256" => {
            Some(db::rfc5912::SHA_256_WITH_RSA_ENCRYPTION)
        }
        "http://www.w3.org/2001/04/xmldsig-more#rsa-sha384" => {
            Some(db::rfc5912::SHA_384_WITH_RSA_ENCRYPTION)
        }
        "http://www.w3.org/2001/04/xmldsig-more#rsa-sha512" => {
            Some(db::rfc5912::SHA_512_WITH_RSA_ENCRYPTION)
        }
        "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha256" => {
            Some(db::rfc5912::ECDSA_WITH_SHA_256)
        }
        "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha384" => {
            Some(db::rfc5912::ECDSA_WITH_SHA_384)
        }
        _ => None,
    }
}

/// The content a reference resolves to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dereferenced {
    /// The whole document.
    Document,
    /// The subtree of one element.
    Element(NodeId),
}

/// The child elements of `parent` named `local` in one of `namespaces`.
fn children_named<'t>(
    tree: &'t Tree,
    parent: NodeId,
    namespaces: &'t [&'t str],
    local: &'t str,
) -> impl Iterator<Item = NodeId> + 't {
    tree.child_elements(parent)
        .filter(move |(_, child)| namespaces.contains(&child.namespace()) && child.local() == local)
        .map(|(id, _)| id)
}

/// The only child element of `parent` named `local` in one of `namespaces`,
/// `None` when absent.
fn optional_child(
    tree: &Tree,
    parent: NodeId,
    namespaces: &[&str],
    local: &str,
    element: SignatureElement,
) -> Result<Option<NodeId>, SignedError> {
    let mut matches = children_named(tree, parent, namespaces, local);
    let first = matches.next();
    if matches.next().is_some() {
        return Err(SignedError::MalformedSignature(SignatureDefect::Repeated(
            element,
        )));
    }
    Ok(first)
}

/// The only child element of `parent` named `local` in one of `namespaces`.
fn required_child(
    tree: &Tree,
    parent: NodeId,
    namespaces: &[&str],
    local: &str,
    element: SignatureElement,
) -> Result<NodeId, SignedError> {
    optional_child(tree, parent, namespaces, local, element)?.ok_or(
        SignedError::MalformedSignature(SignatureDefect::Missing(element)),
    )
}

/// The `Algorithm` of the element `id`.
fn algorithm(tree: &Tree, id: NodeId, element: SignatureElement) -> Result<&str, SignedError> {
    tree.element(id)
        .and_then(|found| found.attribute("Algorithm"))
        .ok_or(SignedError::MalformedSignature(
            SignatureDefect::MissingAlgorithm(element),
        ))
}

/// The `base64` text of the element `id`, decoded, white space ignored as
/// `xs:base64Binary` allows.
fn base64_text(tree: &Tree, id: NodeId, element: SignatureElement) -> Result<Vec<u8>, SignedError> {
    let text: String = tree
        .text(id)
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    base64::engine::general_purpose::STANDARD
        .decode(text)
        .map_err(|_| SignedError::MalformedSignature(SignatureDefect::NotBase64(element)))
}

/// The `ds:SignedInfo` of the signature `signature`.
fn signed_info(tree: &Tree, signature: NodeId) -> Result<NodeId, SignedError> {
    required_child(
        tree,
        signature,
        DSIG,
        "SignedInfo",
        SignatureElement::SignedInfo,
    )
}

/// The `ds:Reference` elements of `signed_info`, at least one.
fn references(tree: &Tree, signed_info: NodeId) -> Result<Vec<NodeId>, SignedError> {
    let references: Vec<NodeId> = children_named(tree, signed_info, DSIG, "Reference").collect();
    if references.is_empty() {
        return Err(SignedError::MalformedSignature(SignatureDefect::Missing(
            SignatureElement::Reference,
        )));
    }
    Ok(references)
}

/// The element carrying the attribute `Id`, `ID` or `id` with the value `id`,
/// refused when no element or more than one does.
fn element_by_id(tree: &Tree, id: &str) -> Result<NodeId, SignedError> {
    let mut matches = tree
        .elements()
        .filter(|(_, element)| {
            ["Id", "ID", "id"]
                .iter()
                .any(|name| element.attribute(name) == Some(id))
        })
        .map(|(node, _)| node);
    let found = matches
        .next()
        .ok_or_else(|| SignedError::UnresolvedReference(id.to_owned()))?;
    if matches.next().is_some() {
        return Err(SignedError::AmbiguousId(id.to_owned()));
    }
    Ok(found)
}

/// The content the `uri` of a reference names, with its name for errors.
fn dereference(tree: &Tree, uri: &str) -> Result<(Dereferenced, ReferenceTarget), SignedError> {
    if uri.is_empty() {
        return Ok((Dereferenced::Document, ReferenceTarget::Document));
    }
    match uri.strip_prefix('#') {
        Some(id) if !id.is_empty() && !id.starts_with("xpointer(") => {
            let element = element_by_id(tree, id)?;
            Ok((
                Dereferenced::Element(element),
                ReferenceTarget::Element(id.to_owned()),
            ))
        }
        _ => Err(SignedError::UnsupportedReference(Some(uri.to_owned()))),
    }
}

/// Whether the `xmldsig-filter2` transform `transform` subtracts every
/// `ds:Signature` and nothing else.
fn subtracts_signatures(tree: &Tree, transform: NodeId) -> Result<bool, SignedError> {
    let xpath = required_child(
        tree,
        transform,
        &[FILTER2],
        "XPath",
        SignatureElement::XPath,
    )?;
    if tree
        .element(xpath)
        .and_then(|element| element.attribute("Filter"))
        != Some("subtract")
    {
        return Ok(false);
    }
    let text = tree.text(xpath);
    let Some((prefix, "Signature")) = text
        .trim()
        .strip_prefix("/descendant::")
        .and_then(|rest| rest.split_once(':'))
    else {
        return Ok(false);
    };
    Ok(!prefix.is_empty()
        && tree
            .namespace_of(xpath, prefix)
            .is_some_and(|namespace| DSIG.contains(&namespace)))
}

/// The parameters of the canonicalization `method` the element `id` names,
/// the `InclusiveNamespaces PrefixList` of the exclusive algorithm with the
/// default namespace as the empty prefix, or `None` when the element carries
/// content the method does not take.
fn canonicalization_prefixes(tree: &Tree, id: NodeId, method: Method) -> Option<Vec<&str>> {
    let mut children = tree.child_elements(id);
    let Some((_, list)) = children.next() else {
        return Some(Vec::new());
    };
    if method == Method::Inclusive
        || children.next().is_some()
        || list.namespace() != EXCLUSIVE
        || list.local() != "InclusiveNamespaces"
    {
        return None;
    }
    Some(
        list.attribute("PrefixList")?
            .split_ascii_whitespace()
            .map(|prefix| if prefix == "#default" { "" } else { prefix })
            .collect(),
    )
}

/// Dereferences, transforms and digests the `ds:Reference` `reference` of
/// the `ds:Signature` `signature`, and checks its `ds:DigestValue`.
///
/// The content of a `""` or `#id` URI carries no comments, so a
/// canonicalization with comments writes none.
fn check_reference(
    tree: &Tree,
    signature: NodeId,
    reference: NodeId,
) -> Result<Dereferenced, SignedError> {
    let uri = tree
        .element(reference)
        .and_then(|element| element.attribute("URI"))
        .ok_or(SignedError::UnsupportedReference(None))?;
    let (dereferenced, target) = dereference(tree, uri)?;
    let mut excluded = Vec::new();
    let mut canonicalization = None;
    if let Some(transforms) = optional_child(
        tree,
        reference,
        DSIG,
        "Transforms",
        SignatureElement::Transforms,
    )? {
        for transform in children_named(tree, transforms, DSIG, "Transform") {
            let algorithm = algorithm(tree, transform, SignatureElement::Transform)?;
            let unsupported = || SignedError::UnsupportedTransform(algorithm.to_owned());
            if canonicalization.is_some() {
                return Err(unsupported());
            }
            match algorithm {
                ENVELOPED if tree.child_elements(transform).next().is_none() => {
                    excluded.push(signature);
                }
                FILTER2 if subtracts_signatures(tree, transform)? => excluded.extend(
                    tree.elements()
                        .filter(|(_, element)| {
                            DSIG.contains(&element.namespace()) && element.local() == "Signature"
                        })
                        .map(|(id, _)| id),
                ),
                _ => {
                    let (method, _) = canonicalization_method(algorithm).ok_or_else(unsupported)?;
                    let prefixes = canonicalization_prefixes(tree, transform, method)
                        .ok_or_else(unsupported)?;
                    canonicalization = Some((method, prefixes));
                }
            }
        }
    }
    let digest_method = required_child(
        tree,
        reference,
        DSIG,
        "DigestMethod",
        SignatureElement::DigestMethod,
    )?;
    let digest = DigestAlgorithm::from_uri(algorithm(
        tree,
        digest_method,
        SignatureElement::DigestMethod,
    )?)?;
    let digest_value = required_child(
        tree,
        reference,
        DSIG,
        "DigestValue",
        SignatureElement::DigestValue,
    )?;
    let expected = base64_text(tree, digest_value, SignatureElement::DigestValue)?;
    let (method, prefixes) = canonicalization.unwrap_or((Method::Inclusive, Vec::new()));
    let algorithm = Canonicalization {
        method,
        comments: false,
        inclusive_prefixes: &prefixes,
    };
    let subset = match dereferenced {
        Dereferenced::Document => Subset::Document {
            excluded: &excluded,
        },
        Dereferenced::Element(apex) => Subset::Element {
            apex,
            excluded: &excluded,
        },
    };
    if digest.digest(&canonicalize(tree, subset, algorithm)) != expected {
        return Err(SignedError::ReferenceDigestMismatch(target));
    }
    Ok(dereferenced)
}

/// The `xades:SignedProperties` of the `xades:QualifyingProperties` in a
/// `ds:Object` of the signature `signature`.
fn signed_properties(tree: &Tree, signature: NodeId) -> Result<NodeId, SignedError> {
    let mut found = children_named(tree, signature, DSIG, "Object")
        .flat_map(|object| children_named(tree, object, XADES, "QualifyingProperties"))
        .flat_map(|qualifying| children_named(tree, qualifying, XADES, "SignedProperties"));
    let signed_properties = found.next().ok_or(SignedError::MissingSignedProperties)?;
    if found.next().is_some() {
        return Err(SignedError::MalformedSignature(SignatureDefect::Repeated(
            SignatureElement::SignedProperties,
        )));
    }
    Ok(signed_properties)
}

/// A certificate of `ds:KeyInfo` with the `DER` bytes it was read from.
#[derive(Debug)]
struct KeyInfoCertificate {
    /// The `DER` encoding, as carried.
    der: Vec<u8>,
    /// The decoded certificate.
    certificate: Certificate,
}

/// The `ds:X509Certificate` elements of the `ds:X509Data` of `key_info`, at
/// least one.
fn key_info_certificates(
    tree: &Tree,
    key_info: NodeId,
) -> Result<Vec<KeyInfoCertificate>, SignedError> {
    let mut certificates = Vec::new();
    for x509_data in children_named(tree, key_info, DSIG, "X509Data") {
        for element in children_named(tree, x509_data, DSIG, "X509Certificate") {
            let der = base64_text(tree, element, SignatureElement::X509Certificate)?;
            let certificate = Certificate::from_der(&der).map_err(|_| {
                SignedError::MalformedSignature(SignatureDefect::NotDer(
                    SignatureElement::X509Certificate,
                ))
            })?;
            certificates.push(KeyInfoCertificate { der, certificate });
        }
    }
    if certificates.is_empty() {
        return Err(SignedError::MalformedSignature(SignatureDefect::Missing(
            SignatureElement::X509Certificate,
        )));
    }
    Ok(certificates)
}

/// The version of a signing certificate property.
#[derive(Clone, Copy, Debug)]
enum SigningCertificateVersion {
    /// `xades:SigningCertificate`, its `xades:IssuerSerial` a name string
    /// and a decimal serial number.
    V1,
    /// `xades:SigningCertificateV2`, its optional `xades:IssuerSerialV2` the
    /// `base64` of a `DER` `IssuerSerial`.
    V2,
}

/// The signer among `certificates`.
///
/// With a signing certificate property the signer is the first certificate
/// every present property references. Without one, `ds:KeyInfo` must be
/// signed and carry a single certificate, which is the signer.
fn select_signer<'c>(
    tree: &Tree,
    signed_properties: NodeId,
    certificates: &'c [KeyInfoCertificate],
    key_info_signed: bool,
) -> Result<&'c KeyInfoCertificate, SignedError> {
    let properties = optional_child(
        tree,
        signed_properties,
        XADES,
        "SignedSignatureProperties",
        SignatureElement::SignedProperties,
    )?;
    let property = |local, element| {
        properties.map_or(Ok(None), |properties| {
            optional_child(tree, properties, XADES, local, element)
        })
    };
    let v1 = property("SigningCertificate", SignatureElement::SigningCertificate)?;
    let v2 = property(
        "SigningCertificateV2",
        SignatureElement::SigningCertificateV2,
    )?;
    if v1.is_none() && v2.is_none() {
        if !key_info_signed {
            return Err(SignedError::SignerCertificateNotProtected);
        }
        return match certificates {
            [only] => Ok(only),
            _ => Err(SignedError::MalformedSignature(SignatureDefect::Repeated(
                SignatureElement::X509Certificate,
            ))),
        };
    }
    for candidate in certificates {
        if property_references(tree, v1, candidate, SigningCertificateVersion::V1)?
            && property_references(tree, v2, candidate, SigningCertificateVersion::V2)?
        {
            return Ok(candidate);
        }
    }
    Err(SignedError::SigningCertificateMismatch)
}

/// Whether the signing certificate property `property`, when present, has
/// an `xades:Cert` referencing `candidate`.
fn property_references(
    tree: &Tree,
    property: Option<NodeId>,
    candidate: &KeyInfoCertificate,
    version: SigningCertificateVersion,
) -> Result<bool, SignedError> {
    let Some(property) = property else {
        return Ok(true);
    };
    for cert in children_named(tree, property, XADES, "Cert") {
        if cert_references(tree, cert, candidate, version)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether the `xades:Cert` `cert` references `candidate`, by the digest of
/// its `DER` encoding and by its issuer and serial number when present.
fn cert_references(
    tree: &Tree,
    cert: NodeId,
    candidate: &KeyInfoCertificate,
    version: SigningCertificateVersion,
) -> Result<bool, SignedError> {
    let cert_digest = required_child(
        tree,
        cert,
        XADES,
        "CertDigest",
        SignatureElement::CertDigest,
    )?;
    let method = required_child(
        tree,
        cert_digest,
        DSIG,
        "DigestMethod",
        SignatureElement::DigestMethod,
    )?;
    let digest =
        DigestAlgorithm::from_uri(algorithm(tree, method, SignatureElement::DigestMethod)?)?;
    let value = required_child(
        tree,
        cert_digest,
        DSIG,
        "DigestValue",
        SignatureElement::DigestValue,
    )?;
    if digest.digest(&candidate.der) != base64_text(tree, value, SignatureElement::DigestValue)? {
        return Ok(false);
    }
    let tbs = candidate.certificate.tbs_certificate();
    match version {
        SigningCertificateVersion::V1 => {
            let issuer_serial = required_child(
                tree,
                cert,
                XADES,
                "IssuerSerial",
                SignatureElement::IssuerSerial,
            )?;
            let name = required_child(
                tree,
                issuer_serial,
                DSIG,
                "X509IssuerName",
                SignatureElement::X509IssuerName,
            )?;
            let serial = required_child(
                tree,
                issuer_serial,
                DSIG,
                "X509SerialNumber",
                SignatureElement::X509SerialNumber,
            )?;
            Ok(issuer_name_matches(&tree.text(name), tbs.issuer())
                && decimal_serial_matches(&tree.text(serial), tbs.serial_number().as_bytes()))
        }
        SigningCertificateVersion::V2 => {
            let Some(element) = optional_child(
                tree,
                cert,
                XADES,
                "IssuerSerialV2",
                SignatureElement::IssuerSerialV2,
            )?
            else {
                return Ok(true);
            };
            let der = base64_text(tree, element, SignatureElement::IssuerSerialV2)?;
            let issuer_serial = IssuerSerial::from_der(&der).map_err(|_| {
                SignedError::MalformedSignature(SignatureDefect::NotDer(
                    SignatureElement::IssuerSerialV2,
                ))
            })?;
            Ok(issuer_serial_matches(
                Some(&issuer_serial),
                &candidate.certificate,
            ))
        }
    }
}

/// Whether the `RFC 4514` string `text` names `issuer`.
///
/// White space after the separators between attributes is dropped first,
/// the `RFC 2253` spelling many signers write.
fn issuer_name_matches(text: &str, issuer: &Name) -> bool {
    let mut compact = String::with_capacity(text.len());
    let mut escaped = false;
    let mut after_separator = false;
    for character in text.trim().chars() {
        if after_separator && character == ' ' {
            continue;
        }
        after_separator = !escaped && matches!(character, ',' | '+');
        escaped = !escaped && character == '\\';
        compact.push(character);
    }
    Name::from_str(&compact).is_ok_and(|name| names_match(&name, issuer))
}

/// Whether the decimal `text` is the positive serial number whose
/// big-endian bytes are `serial`.
fn decimal_serial_matches(text: &str, serial: &[u8]) -> bool {
    let text = text.trim();
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    // The serial in base 10, least significant digit first. A digit is only
    // pushed for a non-zero carry, so the most significant one is non-zero
    // unless the serial is zero.
    let mut digits: Vec<u8> = vec![0];
    for &byte in serial {
        let mut carry = u16::from(byte);
        for digit in &mut digits {
            let value = u16::from(*digit) * 256 + carry;
            [*digit, _] = (value % 10).to_le_bytes();
            carry = value / 10;
        }
        while carry > 0 {
            let [digit, _] = (carry % 10).to_le_bytes();
            digits.push(digit);
            carry /= 10;
        }
    }
    let written = text.trim_start_matches('0');
    let written = if written.is_empty() { "0" } else { written };
    written.len() == digits.len()
        && written
            .bytes()
            .rev()
            .zip(&digits)
            .all(|(character, digit)| character - b'0' == *digit)
}

/// The `xades:SigningTime` of `signed_properties`, when present.
fn signing_time_property(
    tree: &Tree,
    signed_properties: NodeId,
) -> Result<Option<Timestamp>, SignedError> {
    let Some(properties) = optional_child(
        tree,
        signed_properties,
        XADES,
        "SignedSignatureProperties",
        SignatureElement::SignedProperties,
    )?
    else {
        return Ok(None);
    };
    let Some(time) = optional_child(
        tree,
        properties,
        XADES,
        "SigningTime",
        SignatureElement::SignedProperties,
    )?
    else {
        return Ok(None);
    };
    // An `xs:dateTime` without a zone names no instant.
    tree.text(time)
        .trim()
        .parse::<Timestamp>()
        .map(Some)
        .map_err(|_| SignedError::MalformedSigningTime)
}

/// Checks the `ds:SignatureValue` of `signature` over the canonical
/// `ds:SignedInfo` under the key of `signer`.
fn check_signature_value(
    tree: &Tree,
    signature: NodeId,
    signed_info: NodeId,
    signer: &Certificate,
    end_entity: &EndEntityCert<'_>,
) -> Result<(), SignedError> {
    let method_element = required_child(
        tree,
        signed_info,
        DSIG,
        "CanonicalizationMethod",
        SignatureElement::CanonicalizationMethod,
    )?;
    let method_uri = algorithm(
        tree,
        method_element,
        SignatureElement::CanonicalizationMethod,
    )?;
    let unsupported = || SignedError::UnsupportedAlgorithm(method_uri.to_owned());
    let (method, comments) = canonicalization_method(method_uri).ok_or_else(unsupported)?;
    let prefixes =
        canonicalization_prefixes(tree, method_element, method).ok_or_else(unsupported)?;
    let canonical = canonicalize(
        tree,
        Subset::Element {
            apex: signed_info,
            excluded: &[],
        },
        Canonicalization {
            method,
            comments,
            inclusive_prefixes: &prefixes,
        },
    );
    let signature_element = required_child(
        tree,
        signed_info,
        DSIG,
        "SignatureMethod",
        SignatureElement::SignatureMethod,
    )?;
    let signature_uri = algorithm(tree, signature_element, SignatureElement::SignatureMethod)?;
    let unsupported = || SignedError::UnsupportedAlgorithm(signature_uri.to_owned());
    let oid = signature_method(signature_uri).ok_or_else(unsupported)?;
    let verification =
        signature_algorithm(&oid, None, ecdsa_curve(signer).as_ref()).map_err(|_| unsupported())?;
    let value_element = required_child(
        tree,
        signature,
        DSIG,
        "SignatureValue",
        SignatureElement::SignatureValue,
    )?;
    let raw = base64_text(tree, value_element, SignatureElement::SignatureValue)?;
    let value = if oid == db::rfc5912::ECDSA_WITH_SHA_256 || oid == db::rfc5912::ECDSA_WITH_SHA_384
    {
        ecdsa_der(&raw).ok_or(SignedError::SignatureInvalid)?
    } else {
        raw
    };
    end_entity
        .verify_signature(verification, &canonical, &value)
        .map_err(|error| match error {
            WebpkiError::InvalidSignatureForPublicKey => SignedError::SignatureInvalid,
            _ => unsupported(),
        })
}

/// The `DER` `ECDSA-Sig-Value` of an `XMLDSig` `ECDSA` signature value, the
/// `r || s` concatenation of RFC 4051, for the `P-256` and `P-384` sizes.
fn ecdsa_der(raw: &[u8]) -> Option<Vec<u8>> {
    if raw.len() != 64 && raw.len() != 96 {
        return None;
    }
    let mut body = Vec::with_capacity(raw.len() + 6);
    for integer in raw.chunks_exact(raw.len() / 2) {
        let first = integer
            .iter()
            .position(|&byte| byte != 0)
            .unwrap_or(integer.len() - 1);
        let magnitude = &integer[first..];
        let pad = magnitude.first().is_some_and(|&byte| byte >= 0x80);
        body.push(0x02);
        body.push(u8::try_from(magnitude.len() + usize::from(pad)).ok()?);
        if pad {
            body.push(0);
        }
        body.extend_from_slice(magnitude);
    }
    let mut der = vec![0x30, u8::try_from(body.len()).ok()?];
    der.extend(body);
    Some(der)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "test code: the expect message names the failing case"
)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    /// The repository path of `relative`.
    fn path(relative: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    /// Whether each reference of each signature of the file at `file`
    /// digests to its `ds:DigestValue`, in document order.
    fn reference_matches(file: &Path) -> Vec<bool> {
        let input = std::fs::read(file).expect("the fixture reads");
        let tree = Tree::parse(&input).expect("the fixture parses");
        let signatures: Vec<NodeId> = tree
            .elements()
            .filter(|(_, element)| {
                DSIG.contains(&element.namespace()) && element.local() == "Signature"
            })
            .map(|(id, _)| id)
            .collect();
        let mut matches = Vec::new();
        for signature in signatures {
            let signed_info =
                signed_info(&tree, signature).expect("the signature has a SignedInfo");
            for reference in references(&tree, signed_info).expect("the SignedInfo has references")
            {
                let outcome = check_reference(&tree, signature, reference);
                assert!(
                    matches!(
                        outcome,
                        Ok(_) | Err(SignedError::ReferenceDigestMismatch(_))
                    ),
                    "{}: {outcome:?}",
                    file.display()
                );
                matches.push(outcome.is_ok());
            }
        }
        matches
    }

    #[test]
    fn real_signatures_digest_their_references() {
        let all_match: &[(&str, &[bool])] = &[
            (
                "tests/corpus/slamdunk/ok_ITHVQWPH73P42H501Y_X0024.xml",
                &[true; 3],
            ),
            ("tests/fixtures/reader_xpath/default_ns.xml", &[true; 3]),
            ("tests/fixtures/reader_xpath/prefixed_ns.xml", &[true; 3]),
            ("tests/fixtures/TSL-IT.xml", &[true; 2]),
            (
                "tests/fixtures/external/tsl-it/TSL-IT-20170610215159.xml",
                &[true; 2],
            ),
            (
                "tests/fixtures/external/tsl-it/TSL-IT-20211021145639.xml",
                &[true; 2],
            ),
            (
                "tests/fixtures/external/tsl-it/TSL-IT-20220121051644.xml",
                &[true; 2],
            ),
            (
                "tests/fixtures/external/tsl-it/TSL-IT-20231230095405.xml",
                &[true; 2],
            ),
            (
                "tests/fixtures/external/tsl-it/TSL-IT-20241224070933.xml",
                &[true; 2],
            ),
            ("tests/fixtures/external/lotl/eu-lotl.xml", &[true; 2]),
        ];
        for (relative, expected) in all_match {
            assert_eq!(reference_matches(&path(relative)), *expected, "{relative}");
        }
    }

    /// The document references digest the `xml-stylesheet` instruction
    /// before the document element too.
    #[test]
    fn the_specification_samples_digest_their_references() {
        for kind in ["AT", "DT", "MC", "NE", "NS", "RC", "SE"] {
            let relative = format!("spec/samples/IT01234567890_11111_{kind}_001.xml");
            assert_eq!(reference_matches(&path(&relative)), [true; 3], "{relative}");
        }
    }

    #[test]
    fn placeholder_signatures_digest_nothing_they_reference() {
        let mut files = vec![
            path("tests/corpus/merge-invoices/02_invoice.xml"),
            path("tests/corpus/merge-invoices/03_invoice.xml"),
            path("tests/corpus/merge-invoices/05_invoice.xml"),
        ];
        for directory in [
            "tests/corpus/gobl-sdi/fatturapa.gobl",
            "tests/corpus/gobl-sdi/gobl.fatturapa/out",
        ] {
            for entry in std::fs::read_dir(path(directory)).expect("the corpus directory lists") {
                let file = entry.expect("the corpus entry reads").path();
                let signed = std::fs::read(&file)
                    .is_ok_and(|bytes| bytes.windows(14).any(|window| window == b"SignatureValue"));
                if signed {
                    files.push(file);
                }
            }
        }
        assert!(files.len() > 30, "the gobl-sdi signed invoices are found");
        for file in files {
            let matches = reference_matches(&file);
            assert!(
                !matches.is_empty() && matches.iter().all(|matched| !matched),
                "{}",
                file.display()
            );
        }
    }

    #[test]
    fn decimal_serials_compare_by_value() {
        assert!(decimal_serial_matches("0", &[0]));
        assert!(decimal_serial_matches("255", &[0, 0xff]));
        assert!(decimal_serial_matches(" 0065536 ", &[1, 0, 0]));
        assert!(decimal_serial_matches(
            "340282366920938463463374607431768211455",
            &[0xff; 16]
        ));
        assert!(!decimal_serial_matches("256", &[0xff]));
        assert!(!decimal_serial_matches("-1", &[1]));
        assert!(!decimal_serial_matches("", &[0]));
    }

    #[test]
    fn issuer_names_match_with_or_without_spaces_after_separators() {
        let issuer = Name::from_str("CN=Test CA,O=Example,C=IT").expect("the name parses");
        assert!(issuer_name_matches("CN=Test CA, O=Example, C=IT", &issuer));
        assert!(issuer_name_matches("CN=Test CA,O=Example,C=IT", &issuer));
        assert!(!issuer_name_matches(
            "CN=Other CA, O=Example, C=IT",
            &issuer
        ));
        assert!(!issuer_name_matches(
            r"CN=Test CA\, O=Example, C=IT",
            &issuer
        ));
        let escaped = Name::from_str(r"CN=Test\, CA,O=Example").expect("the name parses");
        assert!(issuer_name_matches(r"CN=Test\, CA, O=Example", &escaped));
    }

    #[test]
    fn ecdsa_values_become_der() {
        let mut raw = [0u8; 64];
        raw[0] = 0x80;
        raw[63] = 0x01;
        let der = ecdsa_der(&raw).expect("the size is a P-256 one");
        assert_eq!(&der[..5], &[0x30, 0x26, 0x02, 0x21, 0x00]);
        assert_eq!(&der[der.len() - 3..], &[0x02, 0x01, 0x01]);
        assert_eq!(ecdsa_der(&[1; 63]), None);
        assert_eq!(
            ecdsa_der(&[0; 64]),
            Some(vec![0x30, 0x06, 0x02, 0x01, 0x00, 0x02, 0x01, 0x00])
        );
    }
}
