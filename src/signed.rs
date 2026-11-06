//! Unwrapping and verification of signed `.xml.p7m` files.
//!
//! A signed `FatturaPA` document is a `CAdES` `CMS SignedData` structure
//! (RFC 5652) around the `XML`. The input is raw `DER` or `base64` text, the
//! two forms SDI and mailers deliver. Real signers also write the structure
//! in `BER`, with indefinite length constructs and an `eContent` split into
//! chunks, which the module re-encodes as `DER` with `rasn` before the
//! verification.
//!
//! [`verify`] checks every `SignerInfo` of the structure. For each one it
//! checks that the signed attributes carry a `content-type` matching the
//! encapsulated content and a `message-digest` matching the digest of the
//! content, verifies the signature over the `DER` of the signed attributes
//! with the signer's public key (`RSA PKCS#1 v1.5` and `ECDSA` on `P-256`
//! and `P-384`, with `SHA-256`, `SHA-384` and `SHA-512`), checks the signer
//! certificate's `keyUsage` and `ESS signing-certificate` and
//! `signing-certificate-v2` signed attributes when present, and builds the
//! chain from the signer certificate to one of the trust anchors.
//!
//! The hash of the signed attributes and of the content is taken from the
//! `digestAlgorithm` of the `SignerInfo`. When the signature algorithm names
//! its own hash, the two must agree; a signature algorithm that names no
//! hash, like `rsaEncryption`, relies on the `digestAlgorithm` alone.
//!
//! The chain is verified with `rustls-webpki`. Every link is checked for the
//! issuer/subject match, the signature of the lower certificate under the
//! issuer's key, the validity of every certificate at the signing time, the
//! `CA` basic constraint of every issuer, and the `pathLenConstraint`. The
//! signer certificate is accepted for the signing purpose when its
//! `keyUsage` extension, when present, carries `digitalSignature` or
//! `nonRepudiation`, and when its extended key usage extension, when
//! present, carries `id-kp-emailProtection`, `id-kp-documentSigning`, the
//! Microsoft `document signing` purpose or `anyExtendedKeyUsage`. The trust
//! anchors are identified by their subject and public key; their own
//! validity is not checked. `rustls-webpki` accepts at most six intermediate
//! certificates.
//!
//! Revocation checking (`CRL` and `OCSP`) is not implemented. A caller that
//! needs it performs it itself, for example against the `CRL` distribution
//! points of the chain.

use std::borrow::Cow;
use std::time::Duration;

use base64::Engine;
use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
use cms::content_info::ContentInfo;
use cms::signed_data::{SignedData, SignerIdentifier, SignerInfo};
use const_oid::db;
use der::asn1::{AnyRef, ObjectIdentifier, OctetString};
use der::{Any, Decode, DecodeValue, Encode, FixedTag, Header, Reader, Tag};
use digest::Digest;
use jiff::Timestamp;
use rustls_pki_types::{CertificateDer, SignatureVerificationAlgorithm, TrustAnchor, UnixTime};
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};
use spki::AlgorithmIdentifierOwned;
use webpki::{
    EndEntityCert, Error as WebpkiError, ExtendedKeyUsageValidator, KeyPurposeId, KeyPurposeIdIter,
    KeyUsage as WebpkiKeyUsage, RequiredEkuNotFoundContext, anchor_from_trusted_cert,
};
use x509_cert::Certificate;
use x509_cert::attr::Attributes;
use x509_cert::ext::pkix::KeyUsage;
use x509_cert::time::Time;

use crate::Document;

/// The ETSI ESS `signing-certificate` attribute.
const ESS_SIGNING_CERTIFICATE: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.3.6.1.4.1.41411.1.1.1");

/// The ETSI ESS `signing-certificate-v2` attribute.
const ESS_SIGNING_CERTIFICATE_V2: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.3.6.1.4.1.41411.1.1.3");

/// The named curve `P-384`.
const SECP_384_R_1: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.132.0.34");

/// The `id-kp-emailProtection` extended key usage.
const EMAIL_PROTECTION: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.4");

/// The `id-kp-documentSigning` extended key usage, RFC 9336.
const DOCUMENT_SIGNING: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.36");

/// The Microsoft `document signing` extended key usage.
const MS_DOCUMENT_SIGNING: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.3.6.1.4.1.311.10.3.12");

/// The `anyExtendedKeyUsage` extended key usage.
const ANY_EXTENDED_KEY_USAGE: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.37.0");

/// The signature verification algorithms trusted for the certificate
/// signatures of a chain.
static CHAIN_SIGNATURE_ALGORITHMS: &[&dyn SignatureVerificationAlgorithm] = &[
    webpki::ring::RSA_PKCS1_2048_8192_SHA256,
    webpki::ring::RSA_PKCS1_2048_8192_SHA384,
    webpki::ring::RSA_PKCS1_2048_8192_SHA512,
    webpki::ring::ECDSA_P256_SHA256,
    webpki::ring::ECDSA_P256_SHA384,
    webpki::ring::ECDSA_P384_SHA256,
    webpki::ring::ECDSA_P384_SHA384,
];

/// The extended key usages accepted on a `FatturaPA` signing certificate.
///
/// A certificate without the extension is accepted. When the extension is
/// present it must carry [`EMAIL_PROTECTION`], [`DOCUMENT_SIGNING`],
/// [`MS_DOCUMENT_SIGNING`] or [`ANY_EXTENDED_KEY_USAGE`].
struct SigningEku;

impl ExtendedKeyUsageValidator for SigningEku {
    fn validate(&self, iter: KeyPurposeIdIter<'_, '_>) -> Result<(), WebpkiError> {
        let mut present: Vec<Vec<usize>> = Vec::new();
        for eku in iter {
            let eku = eku?;
            if is_signing_eku(&eku) {
                return Ok(());
            }
            present.push(eku.to_decoded_oid());
        }
        if present.is_empty() {
            // The extension is absent.
            Ok(())
        } else {
            Err(WebpkiError::RequiredEkuNotFoundContext(
                RequiredEkuNotFoundContext {
                    required: WebpkiKeyUsage::required_if_present(EMAIL_PROTECTION.as_ref()),
                    present,
                },
            ))
        }
    }
}

/// Whether `eku` is one of the extended key usages a signing certificate may
/// carry.
fn is_signing_eku(eku: &KeyPurposeId<'_>) -> bool {
    eku == &KeyPurposeId::new(EMAIL_PROTECTION.as_ref())
        || eku == &KeyPurposeId::new(DOCUMENT_SIGNING.as_ref())
        || eku == &KeyPurposeId::new(MS_DOCUMENT_SIGNING.as_ref())
        || eku == &KeyPurposeId::new(ANY_EXTENDED_KEY_USAGE.as_ref())
}

/// Why a signed file could not be unwrapped or verified.
#[derive(Debug, thiserror::Error)]
pub enum SignedError {
    /// The input is not a `CMS SignedData` structure, neither as `DER` nor
    /// as `base64` text.
    #[error("not a CMS signed file: {0}")]
    NotCms(String),

    /// The `CMS SignedData` carries no encapsulated content.
    #[error("the signed file is detached and carries no encapsulated content")]
    DetachedContent,

    /// The encapsulated content type is not `idData`.
    #[error("unsupported encapsulated content type {0}")]
    UnsupportedContentType(String),

    /// The digest or signature algorithm is not supported.
    #[error("unsupported algorithm {0}")]
    UnsupportedAlgorithm(String),

    /// The `message-digest` signed attribute is missing or does not match
    /// the digest of the content.
    #[error("the message-digest attribute does not match the content")]
    DigestMismatch,

    /// The signature over the signed attributes does not verify.
    #[error("the signature over the signed attributes does not verify")]
    SignatureInvalid,

    /// The signer identifier is not an `IssuerAndSerialNumber`.
    #[error("the signer identifier is not an issuer and serial number")]
    UnsupportedSignerIdentifier,

    /// The signer certificate is not among the certificates of the `CMS`.
    #[error("the signer certificate is not among the certificates of the CMS")]
    SignerCertificateNotFound,

    /// The `ESS signing-certificate` or `signing-certificate-v2` attribute
    /// does not reference the signer certificate.
    #[error("the signing-certificate attribute does not reference the signer certificate")]
    SigningCertificateMismatch,

    /// The `content-type` signed attribute is missing or does not match the
    /// encapsulated content type.
    #[error(
        "the content-type signed attribute is missing or does not match the encapsulated content type"
    )]
    ContentTypeMismatch,

    /// The `digestAlgorithm` of the `SignerInfo` does not agree with the
    /// hash named by its signature algorithm.
    #[error("the digest algorithm does not agree with the signature algorithm")]
    AlgorithmMismatch,

    /// The `signing-time` signed attribute is not a valid time.
    #[error("the signing-time attribute is not a valid time")]
    MalformedSigningTime,

    /// No `SignerInfo` carries a `signing-time` attribute and no time was
    /// supplied.
    #[error("no signing-time attribute and no signing time was supplied")]
    MissingSigningTime,

    /// The signer certificate's `keyUsage` extension does not permit
    /// signing.
    #[error("the signer certificate keyUsage does not permit signing")]
    KeyUsage,

    /// The certificate chain does not verify.
    #[error("the certificate chain does not verify: {0}")]
    Chain(#[from] ChainError),
}

/// Why a certificate chain did not verify.
#[derive(Debug, thiserror::Error)]
pub enum ChainError {
    /// No certificate of the store issues the current one.
    #[error("the issuer of the current certificate is not in the store")]
    IssuerNotInStore,

    /// A certificate link does not verify under the issuer's public key.
    #[error("a certificate link does not verify under the issuer key")]
    InvalidSignature,

    /// A certificate of the chain is not valid yet at the signing time.
    #[error("a chain certificate is not valid yet at the signing time")]
    NotYetValid,

    /// A certificate of the chain is expired at the signing time.
    #[error("a chain certificate is expired at the signing time")]
    Expired,

    /// A certificate that issues another one has no `CA:TRUE` basic
    /// constraint.
    #[error("a certificate issuing another one has no CA basic constraint")]
    NotAca,

    /// A `pathLenConstraint` is exceeded.
    #[error("a pathLenConstraint is exceeded")]
    PathLengthExceeded,

    /// The signer certificate is a `CA` certificate, which cannot sign.
    #[error("the signer certificate is a certificate authority")]
    CaSigner,

    /// The signer certificate's extended key usage does not permit signing.
    #[error("the signer certificate extended key usage does not permit signing")]
    ExtendedKeyUsage,

    /// The key algorithm of a certificate link is not supported.
    #[error("unsupported key algorithm {0}")]
    UnsupportedAlgorithm(String),

    /// The chain does not verify for another reason.
    #[error("the certificate chain does not verify: {0}")]
    Other(String),
}

/// The result of verifying a signed file.
///
/// Every `SignerInfo` of the file is verified. The exposed signer, chain and
/// signing time are the ones of the first `SignerInfo`.
#[derive(Debug)]
pub struct VerifiedSignature {
    /// The encapsulated `XML` content.
    content: Vec<u8>,
    /// The signer certificate.
    signer: Certificate,
    /// The chain from the signer certificate to the trust anchor.
    chain: Vec<Certificate>,
    /// The signing time.
    signing_time: Timestamp,
}

impl VerifiedSignature {
    /// The encapsulated `XML` content.
    #[must_use]
    pub fn content(&self) -> &[u8] {
        &self.content
    }

    /// The signer certificate.
    #[must_use]
    pub const fn signer(&self) -> &Certificate {
        &self.signer
    }

    /// The chain from the signer certificate to the trust anchor, including
    /// both.
    #[must_use]
    pub fn chain(&self) -> &[Certificate] {
        &self.chain
    }

    /// The signing time, the `signing-time` signed attribute of the first
    /// `SignerInfo` or the caller-supplied time.
    #[must_use]
    pub const fn signing_time(&self) -> &Timestamp {
        &self.signing_time
    }

    /// Parses the verified content as the document `D`.
    ///
    /// # Errors
    ///
    /// Fails with [`crate::Error`] when the content is not a well-formed
    /// document of type `D`.
    pub fn document<D: Document>(&self) -> Result<D, crate::Error> {
        D::from_xml_slice(self.content())
    }
}

/// Unwraps a `CAdES` signed file to its encapsulated `XML` content.
///
/// The input is `DER` or `base64` text, with or without line breaks. The
/// signature is not verified.
///
/// # Errors
///
/// Fails with [`SignedError`] when the input is not a `CMS SignedData`
/// structure or carries no encapsulated content.
pub fn unwrap(input: &[u8]) -> Result<Vec<u8>, SignedError> {
    let signed_data = parse_signed_data(input)?;
    encapsulated_content(&signed_data)
}

/// Verifies a `CAdES` signed file and returns its content, signer, chain and
/// signing time.
///
/// `signing_time` is used for the validity checks of a `SignerInfo` without
/// a `signing-time` signed attribute. A `SignerInfo` with neither the
/// attribute nor a supplied time is an error. The trust anchors in
/// `anchors` end the chain. Their own validity is not checked, and the chain
/// is built from the certificates of the `CMS` and the anchors only.
///
/// # Errors
///
/// Fails with [`SignedError`] when the file is not a `CMS SignedData`
/// structure, when no `SignerInfo` verifies, or when no chain reaches a
/// trust anchor.
pub fn verify(
    input: &[u8],
    anchors: &[Certificate],
    signing_time: Option<Timestamp>,
) -> Result<VerifiedSignature, SignedError> {
    let signed_data = parse_signed_data(input)?;
    let content = encapsulated_content(&signed_data)?;
    let cms_certs = cms_certificates(&signed_data);
    let signer_infos = &signed_data.signer_infos.0;
    if signer_infos.is_empty() {
        return Err(SignedError::NotCms(
            "the SignedData has no signer info".to_owned(),
        ));
    }
    let mut outcomes = Vec::with_capacity(signer_infos.len());
    for signer_info in signer_infos.iter() {
        outcomes.push(verify_signer_info(
            signer_info,
            &content,
            &cms_certs,
            anchors,
            signing_time,
        )?);
    }
    let (signer, chain, signing_time) = outcomes.remove(0);
    Ok(VerifiedSignature {
        content,
        signer,
        chain,
        signing_time,
    })
}

/// Parses the input as the `DER` of a `CMS ContentInfo` of `SignedData`.
fn parse_signed_data(input: &[u8]) -> Result<SignedData, SignedError> {
    let der = cms_bytes(input)?;
    let content_info = ContentInfo::from_der(der.as_ref())
        .map_err(|_| SignedError::NotCms("the input is not a CMS structure".to_owned()))?;
    if content_info.content_type != db::rfc5911::ID_SIGNED_DATA {
        return Err(SignedError::NotCms(format!(
            "the content type {} is not SignedData",
            content_info.content_type
        )));
    }
    // The `Any` content holds the `SignedData` value without its outer
    // `SEQUENCE` header, so re-encode the full `TLV` before decoding.
    let signed_data_der = content_info
        .content
        .to_der()
        .map_err(|_| SignedError::NotCms("the CMS SignedData structure is malformed".to_owned()))?;
    SignedData::from_der(&signed_data_der)
        .map_err(|_| SignedError::NotCms("the CMS SignedData structure is malformed".to_owned()))
}

/// The certificates of the `CMS`, in their declared order.
fn cms_certificates(signed_data: &SignedData) -> Vec<Certificate> {
    let mut certificates = Vec::new();
    if let Some(set) = &signed_data.certificates {
        for choice in set.0.iter() {
            if let CertificateChoices::Certificate(certificate) = choice {
                certificates.push(certificate.clone());
            }
        }
    }
    certificates
}

/// The `DER` of the `CMS` structure in `input`, with any `BER` form
/// normalized to `DER`. The input is the raw bytes of the structure or
/// `base64` text, and the bytes are borrowed when they are already strict
/// `DER`.
fn cms_bytes(input: &[u8]) -> Result<Cow<'_, [u8]>, SignedError> {
    if let Some(der) = cms_structure(input) {
        return Ok(der);
    }
    // The bytes are not a `CMS` structure: read them as `base64` text.
    let stripped: Vec<u8> = input
        .iter()
        .copied()
        .filter(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        .collect();
    if stripped.is_empty() || !stripped.iter().copied().all(is_base64_char) {
        return Err(SignedError::NotCms(
            "the input is not a CMS structure".to_owned(),
        ));
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&stripped)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(&stripped))
        .map_err(|_| SignedError::NotCms("the base64 text does not decode".to_owned()))?;
    match cms_structure(&decoded) {
        // The bytes need no rewrite, but they are owned here, so copy them.
        Some(Cow::Borrowed(_)) => Ok(Cow::Owned(decoded)),
        Some(Cow::Owned(der)) => Ok(Cow::Owned(der)),
        None => Err(SignedError::NotCms(
            "the input is not a CMS structure".to_owned(),
        )),
    }
}

/// The `DER` of `input` as a `CMS` structure, with any `BER` form normalized
/// to `DER`, or `None` when the input is not a `CMS` structure.
fn cms_structure(input: &[u8]) -> Option<Cow<'_, [u8]>> {
    // The `cms` crate reads only strict `DER`, but real signers write `BER`.
    let strict = ContentInfo::from_der(input).ok();
    if strict.as_ref().is_some_and(signed_data_parses) {
        return Some(Cow::Borrowed(input));
    }
    // `rasn` reads `BER`; re-encode the `SignedData` as `DER`.
    let content_info = rasn::ber::decode::<rasn_cms::ContentInfo>(input).ok()?;
    let signed_data =
        rasn::ber::decode::<rasn_cms::SignedData>(content_info.content.as_bytes()).ok()?;
    let signed_data_der = rasn::der::encode(&signed_data).ok()?;
    let fresh = rasn_cms::ContentInfo {
        content_type: content_info.content_type,
        content: rasn::types::Any::new(signed_data_der),
    };
    Some(Cow::Owned(rasn::der::encode(&fresh).ok()?))
}

/// Whether the `SignedData` of `content_info` reads as strict `DER`, with a
/// primitive `eContent`.
fn signed_data_parses(content_info: &ContentInfo) -> bool {
    content_info
        .content
        .to_der()
        .is_ok_and(|signed_data_der| SignedData::from_der(&signed_data_der).is_ok())
}

/// A character of the `base64` alphabet, padding included.
const fn is_base64_char(byte: u8) -> bool {
    matches!(
        byte,
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'='
    )
}

/// The encapsulated content of the `SignedData`, or the detached-content
/// error.
fn encapsulated_content(signed_data: &SignedData) -> Result<Vec<u8>, SignedError> {
    let encapsulated = &signed_data.encap_content_info;
    if encapsulated.econtent_type != db::rfc5911::ID_DATA {
        return Err(SignedError::UnsupportedContentType(
            encapsulated.econtent_type.to_string(),
        ));
    }
    encapsulated
        .econtent
        .as_ref()
        .map(|any| any.value().to_vec())
        .ok_or(SignedError::DetachedContent)
}

/// Verifies one `SignerInfo` and returns the signer certificate, the chain
/// and the signing time.
fn verify_signer_info(
    signer_info: &SignerInfo,
    content: &[u8],
    cms_certs: &[Certificate],
    anchors: &[Certificate],
    signing_time: Option<Timestamp>,
) -> Result<(Certificate, Vec<Certificate>, Timestamp), SignedError> {
    let sid = match &signer_info.sid {
        SignerIdentifier::IssuerAndSerialNumber(issuer_and_serial) => issuer_and_serial,
        SignerIdentifier::SubjectKeyIdentifier(_) => {
            return Err(SignedError::UnsupportedSignerIdentifier);
        }
    };
    let signer = cms_certs
        .iter()
        .find(|certificate| {
            certificate.tbs_certificate.issuer == sid.issuer
                && certificate.tbs_certificate.serial_number == sid.serial_number
        })
        .cloned()
        .ok_or(SignedError::SignerCertificateNotFound)?;

    let signed_attrs = signer_info
        .signed_attrs
        .as_ref()
        .ok_or(SignedError::ContentTypeMismatch)?;

    // RFC 5652 section 5.3 requires the `content-type` and the
    // `message-digest` signed attributes of every `SignerInfo`.
    let content_type = attr_value(signed_attrs, db::rfc5911::ID_CONTENT_TYPE)
        .and_then(|value| value.decode_as::<ObjectIdentifier>().ok())
        .ok_or(SignedError::ContentTypeMismatch)?;
    if content_type != db::rfc5911::ID_DATA {
        return Err(SignedError::ContentTypeMismatch);
    }
    let message_digest = attr_value(signed_attrs, db::rfc5911::ID_MESSAGE_DIGEST)
        .ok_or(SignedError::DigestMismatch)?;

    let signing_time = match attr_value(signed_attrs, db::rfc5911::ID_SIGNING_TIME) {
        Some(value) => parse_time(value)?,
        None => signing_time.ok_or(SignedError::MissingSigningTime)?,
    };

    let expected = digest(content, &signer_info.digest_alg.oid)?;
    if message_digest.value() != expected.as_slice() {
        return Err(SignedError::DigestMismatch);
    }

    let signed_attrs_der = signed_attrs
        .to_der()
        .map_err(|_| SignedError::DigestMismatch)?;

    let signer_der = signer
        .to_der()
        .map_err(|_| SignedError::UnsupportedAlgorithm("the signer certificate".to_owned()))?;
    let signer_der = CertificateDer::from(signer_der);
    let end_entity = EndEntityCert::try_from(&signer_der)
        .map_err(|_| SignedError::UnsupportedAlgorithm("the signer certificate".to_owned()))?;
    let algorithm = signature_algorithm(
        &signer_info.signature_algorithm.oid,
        Some(&signer_info.digest_alg.oid),
        ecdsa_curve(&signer).as_ref(),
    )
    .map_err(|check| signature_check_error(&check))?;
    end_entity
        .verify_signature(
            algorithm,
            &signed_attrs_der,
            signer_info.signature.as_bytes(),
        )
        .map_err(|error| match error {
            WebpkiError::InvalidSignatureForPublicKey => SignedError::SignatureInvalid,
            _ => SignedError::UnsupportedAlgorithm(signer_info.signature_algorithm.oid.to_string()),
        })?;

    key_usage_permits_signing(&signer)?;

    verify_ess(signed_attrs, &signer)?;

    let chain = verify_chain(&signer, &end_entity, signing_time, cms_certs, anchors)?;
    Ok((signer, chain, signing_time))
}

/// The value of the first attribute with `oid` in `attrs`, if present.
fn attr_value(attrs: &Attributes, oid: ObjectIdentifier) -> Option<&Any> {
    attrs
        .iter()
        .find(|attr| attr.oid == oid)
        .and_then(|attr| attr.values.iter().next())
}

/// The `SHA-256/384/512` digest of `data` per `oid`, or the unsupported-
/// algorithm error.
fn digest(data: &[u8], oid: &ObjectIdentifier) -> Result<Vec<u8>, SignedError> {
    match *oid {
        db::rfc5912::ID_SHA_256 => Ok(Sha256::digest(data).to_vec()),
        db::rfc5912::ID_SHA_384 => Ok(Sha384::digest(data).to_vec()),
        db::rfc5912::ID_SHA_512 => Ok(Sha512::digest(data).to_vec()),
        _ => Err(SignedError::UnsupportedAlgorithm(oid.to_string())),
    }
}

/// An ASN.1 `Time` attribute value to a `UTC` instant.
///
/// The `UTCTime` and `GeneralizedTime` values are parsed by the library,
/// with its checked forms and its `UTCTime` year window.
fn parse_time(value: &Any) -> Result<Timestamp, SignedError> {
    // `Any` keeps the content without the outer `TLV`; `from_der` needs it back.
    let tlv = value
        .to_der()
        .map_err(|_| SignedError::MalformedSigningTime)?;
    let time = Time::from_der(&tlv).map_err(|_| SignedError::MalformedSigningTime)?;
    let unix = i64::try_from(time.to_unix_duration().as_secs())
        .map_err(|_| SignedError::MalformedSigningTime)?;
    Timestamp::from_second(unix).map_err(|_| SignedError::MalformedSigningTime)
}

/// The `EC` curve of the certificate's public key, when it is an `EC` key.
fn ecdsa_curve(certificate: &Certificate) -> Option<ObjectIdentifier> {
    let spki = &certificate.tbs_certificate.subject_public_key_info;
    if spki.algorithm.oid != db::rfc5912::ID_EC_PUBLIC_KEY {
        return None;
    }
    spki.algorithm
        .parameters
        .as_ref()
        .and_then(|parameters| parameters.decode_as::<ObjectIdentifier>().ok())
}

/// The signature verification algorithm of a `SignerInfo` signature, from
/// its signature algorithm `OID`, its `digestAlgorithm` `OID`, and the
/// curve of the signer's key, or the check that failed.
fn signature_algorithm(
    signature_oid: &ObjectIdentifier,
    digest_oid: Option<&ObjectIdentifier>,
    curve: Option<&ObjectIdentifier>,
) -> Result<&'static dyn SignatureVerificationAlgorithm, SignatureCheck> {
    // A signature algorithm that names its own hash must agree with the
    // `digestAlgorithm`, when the signed structure declares one.
    let named: Option<ObjectIdentifier> = match *signature_oid {
        db::rfc5912::RSA_ENCRYPTION => None,
        db::rfc5912::SHA_256_WITH_RSA_ENCRYPTION | db::rfc5912::ECDSA_WITH_SHA_256 => {
            Some(db::rfc5912::ID_SHA_256)
        }
        db::rfc5912::SHA_384_WITH_RSA_ENCRYPTION | db::rfc5912::ECDSA_WITH_SHA_384 => {
            Some(db::rfc5912::ID_SHA_384)
        }
        db::rfc5912::SHA_512_WITH_RSA_ENCRYPTION | db::rfc5912::ECDSA_WITH_SHA_512 => {
            Some(db::rfc5912::ID_SHA_512)
        }
        _ => return Err(SignatureCheck::UnsupportedAlgorithm(*signature_oid)),
    };
    let digest_oid = match (named, digest_oid) {
        (None, None) => {
            // `rsaEncryption` names no hash; the `digestAlgorithm` is the hash.
            return Err(SignatureCheck::UnsupportedAlgorithm(*signature_oid));
        }
        (None, Some(digest_oid)) => *digest_oid,
        (Some(named), None) => named,
        (Some(named), Some(digest_oid)) => {
            if named != *digest_oid {
                return Err(SignatureCheck::AlgorithmMismatch);
            }
            *digest_oid
        }
    };
    Ok(match (signature_oid, &digest_oid, curve) {
        // `RSA PKCS#1 v1.5`, over a key that is not an `EC` key.
        (signature, digest, None)
            if (*signature == db::rfc5912::RSA_ENCRYPTION
                || *signature == db::rfc5912::SHA_256_WITH_RSA_ENCRYPTION)
                && *digest == db::rfc5912::ID_SHA_256 =>
        {
            webpki::ring::RSA_PKCS1_2048_8192_SHA256
        }
        (signature, digest, None)
            if (*signature == db::rfc5912::RSA_ENCRYPTION
                || *signature == db::rfc5912::SHA_384_WITH_RSA_ENCRYPTION)
                && *digest == db::rfc5912::ID_SHA_384 =>
        {
            webpki::ring::RSA_PKCS1_2048_8192_SHA384
        }
        (signature, digest, None)
            if (*signature == db::rfc5912::RSA_ENCRYPTION
                || *signature == db::rfc5912::SHA_512_WITH_RSA_ENCRYPTION)
                && *digest == db::rfc5912::ID_SHA_512 =>
        {
            webpki::ring::RSA_PKCS1_2048_8192_SHA512
        }
        // `ECDSA`, over the curve of the key.
        (signature, digest, Some(curve))
            if *signature == db::rfc5912::ECDSA_WITH_SHA_256
                && *digest == db::rfc5912::ID_SHA_256
                && *curve == db::rfc5912::SECP_256_R_1 =>
        {
            webpki::ring::ECDSA_P256_SHA256
        }
        (signature, digest, Some(curve))
            if *signature == db::rfc5912::ECDSA_WITH_SHA_256
                && *digest == db::rfc5912::ID_SHA_256
                && *curve == SECP_384_R_1 =>
        {
            webpki::ring::ECDSA_P384_SHA256
        }
        (signature, digest, Some(curve))
            if *signature == db::rfc5912::ECDSA_WITH_SHA_384
                && *digest == db::rfc5912::ID_SHA_384
                && *curve == db::rfc5912::SECP_256_R_1 =>
        {
            webpki::ring::ECDSA_P256_SHA384
        }
        (signature, digest, Some(curve))
            if *signature == db::rfc5912::ECDSA_WITH_SHA_384
                && *digest == db::rfc5912::ID_SHA_384
                && *curve == SECP_384_R_1 =>
        {
            webpki::ring::ECDSA_P384_SHA384
        }
        _ => return Err(SignatureCheck::UnsupportedAlgorithm(*signature_oid)),
    })
}

/// The signer certificate's `keyUsage` extension, when present, must permit
/// signing.
fn key_usage_permits_signing(signer: &Certificate) -> Result<(), SignedError> {
    match signer.tbs_certificate.get::<KeyUsage>() {
        Ok(None) => Ok(()),
        Ok(Some((_, key_usage))) => {
            if key_usage.digital_signature() || key_usage.non_repudiation() {
                Ok(())
            } else {
                Err(SignedError::KeyUsage)
            }
        }
        Err(_) => Err(SignedError::KeyUsage),
    }
}

/// The outcome of a signature algorithm check.
#[derive(Debug)]
enum SignatureCheck {
    /// The algorithm is not supported.
    UnsupportedAlgorithm(ObjectIdentifier),

    /// The digest algorithm does not agree with the hash named by the
    /// signature algorithm.
    AlgorithmMismatch,
}

/// Maps a signature check failure to the signed-file error.
fn signature_check_error(check: &SignatureCheck) -> SignedError {
    match check {
        SignatureCheck::UnsupportedAlgorithm(oid) => {
            SignedError::UnsupportedAlgorithm(oid.to_string())
        }
        SignatureCheck::AlgorithmMismatch => SignedError::AlgorithmMismatch,
    }
}

/// Checks the `ESS signing-certificate` and `signing-certificate-v2`
/// signed attributes, when present, against `signer`.
///
/// The attributes are meaningful only as signed attributes, so the unsigned
/// attributes of the `SignerInfo` are not read for them.
fn verify_ess(signed_attrs: &Attributes, signer: &Certificate) -> Result<(), SignedError> {
    let signer_der = signer
        .to_der()
        .map_err(|_| SignedError::SigningCertificateMismatch)?;
    if let Some(v1) = attr_value(signed_attrs, ESS_SIGNING_CERTIFICATE)
        && !ess_v1_references(v1, &signer_der, signer)
    {
        return Err(SignedError::SigningCertificateMismatch);
    }
    if let Some(v2) = attr_value(signed_attrs, ESS_SIGNING_CERTIFICATE_V2) {
        // `Any` keeps the content without the outer `TLV`; `from_der` needs it back.
        let v2 = v2
            .to_der()
            .map_err(|_| SignedError::SigningCertificateMismatch)?;
        if !ess_v2_references(&v2, &signer_der, signer) {
            return Err(SignedError::SigningCertificateMismatch);
        }
    }
    Ok(())
}

/// Whether the `signing-certificate` attribute references `signer`.
fn ess_v1_references(value: &Any, signer_der: &[u8], signer: &Certificate) -> bool {
    // `Any` keeps the content without the outer `TLV`; `from_der` needs it back.
    let Ok(tlv) = value.to_der() else {
        return false;
    };
    let Ok(entry) = EssSigningCertificateV1::from_der(&tlv) else {
        return false;
    };
    // The v1 hash is `SHA-1` by definition.
    if entry.hash.as_bytes() != Sha1::digest(signer_der).as_slice() {
        return false;
    }
    match entry.issuer_serial {
        Some(issuer_serial) => {
            issuer_serial.issuer == signer.tbs_certificate.issuer
                && issuer_serial.serial_number == signer.tbs_certificate.serial_number
        }
        None => true,
    }
}

/// Whether the `signing-certificate-v2` attribute references `signer`.
fn ess_v2_references(ess_v2: &[u8], signer_der: &[u8], signer: &Certificate) -> bool {
    let entries: EssV2List = match EssV2List::from_der(ess_v2) {
        Ok(entries) => entries,
        Err(_) => return false,
    };
    entries.0.iter().any(|entry| {
        ess_hash_matches(
            entry
                .hash_algorithm
                .as_ref()
                .map(|algorithm| &algorithm.oid),
            &entry.hash,
            signer_der,
        ) && entry.issuer == signer.tbs_certificate.issuer
            && entry.serial == signer.tbs_certificate.serial_number
    })
}

/// Whether `hash` is the hash of `signer_der` per `hash_algorithm`.
fn ess_hash_matches(
    hash_algorithm: Option<&ObjectIdentifier>,
    hash: &OctetString,
    signer_der: &[u8],
) -> bool {
    hash_algorithm_for(hash_algorithm)
        .is_some_and(|family| family.digest(signer_der) == hash.as_bytes())
}

/// The digest family of a `signing-certificate-v2` hash algorithm,
/// `SHA-256` when the algorithm is absent.
fn hash_algorithm_for(oid: Option<&ObjectIdentifier>) -> Option<Hash> {
    match oid {
        None => Some(Hash::Sha256),
        Some(oid) if *oid == db::rfc5912::ID_SHA_256 => Some(Hash::Sha256),
        Some(oid) if *oid == db::rfc5912::ID_SHA_384 => Some(Hash::Sha384),
        Some(oid) if *oid == db::rfc5912::ID_SHA_512 => Some(Hash::Sha512),
        Some(oid) if *oid == db::rfc5912::ID_SHA_1 => Some(Hash::Sha1),
        Some(_) => None,
    }
}

/// A digest family.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Hash {
    /// `SHA-1`.
    Sha1,
    /// `SHA-256`.
    Sha256,
    /// `SHA-384`.
    Sha384,
    /// `SHA-512`.
    Sha512,
}

impl Hash {
    /// The digest of `data`.
    fn digest(self, data: &[u8]) -> Vec<u8> {
        match self {
            Self::Sha1 => Sha1::digest(data).to_vec(),
            Self::Sha256 => Sha256::digest(data).to_vec(),
            Self::Sha384 => Sha384::digest(data).to_vec(),
            Self::Sha512 => Sha512::digest(data).to_vec(),
        }
    }
}

/// The value of a `signing-certificate-v2` attribute, a `SEQUENCE OF`
/// `ESSCertIDv2`, which `SetOfVec` cannot read as it decodes a `SET OF`.
#[derive(Clone, Debug)]
struct EssV2List(Vec<EssSigningCertificate>);

impl FixedTag for EssV2List {
    const TAG: Tag = Tag::Sequence;
}

impl<'a> DecodeValue<'a> for EssV2List {
    fn decode_value<R: Reader<'a>>(reader: &mut R, header: Header) -> der::Result<Self> {
        reader.read_nested(header.length, |reader| {
            let mut entries = Vec::new();
            while !reader.is_finished() {
                entries.push(EssSigningCertificate::decode(reader)?);
            }
            Ok(Self(entries))
        })
    }
}

/// One entry of a `signing-certificate` attribute, an `ESSCertID`.
#[derive(Clone, Debug, der::Sequence)]
struct EssSigningCertificateV1 {
    /// The hash of the certificate `DER` encoding.
    hash: OctetString,
    /// The issuer and serial of the referenced certificate, when present.
    issuer_serial: Option<IssuerAndSerialNumber>,
}

/// One entry of a `signing-certificate-v2` attribute.
#[derive(Clone, Debug, Eq, PartialEq, der::Sequence)]
struct EssSigningCertificate {
    /// The hash algorithm of `hash`, `SHA-256` when absent.
    hash_algorithm: Option<AlgorithmIdentifierOwned>,
    /// The hash of the certificate `DER` encoding.
    hash: OctetString,
    /// The issuer of the referenced certificate.
    issuer: x509_cert::name::Name,
    /// The serial number of the referenced certificate.
    serial: x509_cert::serial_number::SerialNumber,
}

/// The chain from the signer certificate to a trust anchor, verified with
/// `rustls-webpki`.
///
/// `leaf` and `end_entity` are the same certificate; the `Certificate` is
/// needed to exclude the signer from the intermediate pool and to seed the
/// returned chain.
fn verify_chain(
    leaf: &Certificate,
    end_entity: &EndEntityCert<'_>,
    signing_time: Timestamp,
    cms_certs: &[Certificate],
    anchors: &[Certificate],
) -> Result<Vec<Certificate>, ChainError> {
    let intermediates: Vec<CertificateDer<'_>> = cms_certs
        .iter()
        .filter(|certificate| {
            !(certificate.tbs_certificate.issuer == leaf.tbs_certificate.issuer
                && certificate.tbs_certificate.serial_number == leaf.tbs_certificate.serial_number)
        })
        .map(Certificate::to_der)
        .collect::<Result<Vec<Vec<u8>>, _>>()
        .map_err(|_| ChainError::Other("a CMS certificate does not encode".to_owned()))?
        .into_iter()
        .map(CertificateDer::from)
        .collect();
    let anchor_ders: Vec<CertificateDer<'_>> = anchors
        .iter()
        .map(Certificate::to_der)
        .collect::<Result<Vec<Vec<u8>>, _>>()
        .map_err(|_| ChainError::Other("a trust anchor does not encode".to_owned()))?
        .into_iter()
        .map(CertificateDer::from)
        .collect();
    let trust_anchors: Vec<TrustAnchor<'_>> = anchor_ders
        .iter()
        .map(anchor_from_trusted_cert)
        .collect::<Result<_, _>>()
        .map_err(chain_error)?;
    // Pre-epoch instants read as zero, as the validity checks did before.
    let time = UnixTime::since_unix_epoch(Duration::from_secs(
        u64::try_from(signing_time.as_second()).unwrap_or(0),
    ));
    let path = end_entity
        .verify_for_usage(
            CHAIN_SIGNATURE_ALGORITHMS,
            &trust_anchors,
            &intermediates,
            time,
            SigningEku,
            None,
            None,
        )
        .map_err(chain_error)?;
    let mut chain = vec![leaf.clone()];
    for certificate in path.intermediate_certificates() {
        chain.push(
            Certificate::from_der(certificate.der().as_ref())
                .map_err(|_| ChainError::Other("a chain certificate does not decode".to_owned()))?,
        );
    }
    // The anchor's SPKI carries no outer `SEQUENCE` `TLV`, so strip it.
    let anchor = path.anchor();
    let anchor_spki = anchor.subject_public_key_info.as_ref();
    let anchor_cert = anchors
        .iter()
        .find(|certificate| {
            certificate
                .tbs_certificate
                .subject_public_key_info
                .to_der()
                .ok()
                .as_deref()
                .and_then(tlv_content)
                == Some(anchor_spki)
        })
        .cloned()
        .ok_or_else(|| ChainError::Other("the verified trust anchor is not supplied".to_owned()))?;
    chain.push(anchor_cert);
    Ok(chain)
}

/// The content of a `DER` `TLV`, after the tag and length encoding, with the
/// `DER` parser.
fn tlv_content(tlv: &[u8]) -> Option<&[u8]> {
    AnyRef::from_der(tlv).ok().map(AnyRef::value)
}

/// Maps a `webpki` verification error to the chain error.
fn chain_error(error: WebpkiError) -> ChainError {
    match error {
        WebpkiError::CertNotValidYet { .. } => ChainError::NotYetValid,
        WebpkiError::CertExpired { .. } => ChainError::Expired,
        WebpkiError::UnknownIssuer => ChainError::IssuerNotInStore,
        WebpkiError::InvalidSignatureForPublicKey => ChainError::InvalidSignature,
        WebpkiError::EndEntityUsedAsCa => ChainError::NotAca,
        WebpkiError::PathLenConstraintViolated => ChainError::PathLengthExceeded,
        WebpkiError::CaUsedAsEndEntity => ChainError::CaSigner,
        WebpkiError::RequiredEkuNotFoundContext(_) | WebpkiError::EmptyEkuExtension => {
            ChainError::ExtendedKeyUsage
        }
        WebpkiError::UnsupportedSignatureAlgorithmContext(_)
        | WebpkiError::UnsupportedSignatureAlgorithmForPublicKeyContext(_) => {
            ChainError::UnsupportedAlgorithm(error.to_string())
        }
        other => ChainError::Other(other.to_string()),
    }
}
