//! Unwrapping and verification of signed `.xml.p7m` files.
#![expect(
    clippy::expect_used,
    reason = "a test fails loudly on any unexpected fixture"
)]

mod support;

use base64::Engine;
use cms::cert::IssuerAndSerialNumber;
use cms::content_info::{CmsVersion, ContentInfo};
use cms::signed_data::{
    CertificateSet, EncapsulatedContentInfo, SignedData, SignerIdentifier, SignerInfo, SignerInfos,
};
use const_oid::db;
use der::asn1::{ObjectIdentifier, OctetString, SetOfVec};
use der::{Any, Decode, DecodePem, Encode};
use digest::Digest;
use ecdsa::SigningKey;
use ecdsa::signature::hazmat::PrehashSigner;
use fatturapa::signed::{ChainError, SignedError, unwrap, verify};
use fatturapa::{Document, FatturaElettronica};
use jiff::SignedDuration;
use jiff::Timestamp;
use p256::NistP256;
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};
use spki::AlgorithmIdentifierOwned;
use std::time::UNIX_EPOCH;
use x509_cert::Certificate;
use x509_cert::attr::{Attribute, Attributes};
use x509_cert::name::Name;
use x509_cert::serial_number::SerialNumber;

/// The ETSI ESS `signing-certificate` attribute.
const ESS_SIGNING_CERTIFICATE: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.3.6.1.4.1.41411.1.1.1");

/// The ETSI ESS `signing-certificate-v2` attribute.
const ESS_SIGNING_CERTIFICATE_V2: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.3.6.1.4.1.41411.1.1.3");

/// The bytes of the official sample invoice `IT01234567890_FPR01.xml`.
fn invoice() -> Vec<u8> {
    support::sample_bytes("IT01234567890_FPR01.xml")
}

/// The certificate in the fixture file with name `name`.
fn cert(name: &str) -> Certificate {
    Certificate::from_pem(support::fixture_bytes(name).as_slice())
        .expect("the fixture is a PEM certificate")
}

/// One hour after the start of the validity of `cert`, inside the validity
/// of every fixture certificate.
fn signing_time(cert: &Certificate) -> Timestamp {
    let not_before = cert.tbs_certificate.validity.not_before.to_system_time();
    let secs = not_before
        .duration_since(UNIX_EPOCH)
        .expect("after 1970")
        .as_secs();
    Timestamp::from_second(i64::try_from(secs).expect("fits") + 3600).expect("fits")
}

/// The `DER` value length and the size in bytes of the length encoding,
/// from the first length octet at `bytes[pos]`.
fn der_length(bytes: &[u8], pos: usize) -> Option<(usize, usize)> {
    let first = *bytes.get(pos)?;
    if first < 0x80 {
        return Some((usize::from(first), 1));
    }
    let count = usize::from(first & 0x7f);
    if count == 0 || count > 4 || pos + count >= bytes.len() {
        return None;
    }
    let mut len = 0usize;
    for i in 1..=count {
        len = len * 256 + usize::from(*bytes.get(pos + i)?);
    }
    Some((len, count + 1))
}

/// The scalar of the `ec-signer.key` fixture, the first 32-byte `OCTET
/// STRING` of its `SEC1` encoding.
fn ec_signer_scalar() -> Vec<u8> {
    let pem = support::fixture_bytes("ec-signer.key");
    let text = std::str::from_utf8(&pem).expect("the key is UTF-8");
    let body: String = text
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();
    let der = base64::engine::general_purpose::STANDARD
        .decode(body)
        .expect("the key is base64");
    let mut pos = 0usize;
    while let Some(&tag) = der.get(pos) {
        let (len, header) = der_length(&der, pos + 1).expect("a valid DER length");
        if tag == 0x04 && len == 32 {
            return der[pos + 1 + header..pos + 1 + header + 32].to_vec();
        }
        pos += 1;
    }
    panic!("the SEC1 key has no 32-byte scalar");
}

/// One signed attribute with `oid` and the single value of `value_der`.
fn attr(oid: ObjectIdentifier, value_der: &[u8]) -> Attribute {
    let value = Any::from_der(value_der).expect("valid DER");
    Attribute {
        oid,
        values: SetOfVec::try_from(vec![value]).expect("a single value"),
    }
}

/// A `SEQUENCE` of the `DER` `items`, with a definite length encoding.
fn sequence_of(items: Vec<Vec<u8>>) -> Vec<u8> {
    let body: Vec<u8> = items.into_iter().flatten().collect();
    let mut out = vec![0x30];
    if body.len() < 0x80 {
        out.push(u8::try_from(body.len()).expect("below 128"));
    } else {
        let mut bytes = Vec::new();
        let mut len = body.len();
        while len > 0 {
            bytes.push(u8::try_from(len & 0xff).expect("fits"));
            len >>= 8;
        }
        bytes.reverse();
        out.push(0x80 | u8::try_from(bytes.len()).expect("fits"));
        out.extend_from_slice(&bytes);
    }
    out.extend_from_slice(&body);
    out
}

/// A hash family for the in-process signatures.
#[derive(Clone, Copy)]
enum Prehash {
    /// `SHA-256`.
    Sha256,
    /// `SHA-384`.
    Sha384,
    /// `SHA-512`.
    Sha512,
}

impl Prehash {
    /// The digest of `data`.
    fn digest(self, data: &[u8]) -> Vec<u8> {
        match self {
            Self::Sha256 => Sha256::digest(data).to_vec(),
            Self::Sha384 => Sha384::digest(data).to_vec(),
            Self::Sha512 => Sha512::digest(data).to_vec(),
        }
    }
}

/// The digest, signature, and prehash algorithms of an in-process `CMS`.
struct Algs {
    /// The `digestAlgorithm` of the `SignerInfo`.
    digest: ObjectIdentifier,
    /// The signature algorithm of the `SignerInfo`.
    signature: ObjectIdentifier,
    /// The hash of the signed attributes.
    prehash: Prehash,
}

impl Algs {
    /// The hash family of the `digestAlgorithm`.
    fn digest_hash(&self) -> Prehash {
        match self.digest {
            oid if oid == db::rfc5912::ID_SHA_256 => Prehash::Sha256,
            oid if oid == db::rfc5912::ID_SHA_384 => Prehash::Sha384,
            oid if oid == db::rfc5912::ID_SHA_512 => Prehash::Sha512,
            _ => panic!("an unsupported digest algorithm"),
        }
    }
}

/// The default in-process algorithms, `SHA-256` over `P-256`.
const fn default_algs() -> Algs {
    Algs {
        digest: db::rfc5912::ID_SHA_256,
        signature: db::rfc5912::ECDSA_WITH_SHA_256,
        prehash: Prehash::Sha256,
    }
}

/// The signed attributes over `content` hashed with `hash`, with a
/// `signing-time` attribute when `signing_time` is set.
fn signed_attrs(content: &[u8], signing_time: Option<Timestamp>, hash: Prehash) -> Attributes {
    let mut attrs = vec![
        attr(
            db::rfc5911::ID_CONTENT_TYPE,
            db::rfc5911::ID_DATA
                .to_der()
                .expect("the OID encodes")
                .as_slice(),
        ),
        attr(
            db::rfc5911::ID_MESSAGE_DIGEST,
            OctetString::new(hash.digest(content))
                .expect("the digest encodes")
                .to_der()
                .expect("the digest encodes")
                .as_slice(),
        ),
    ];
    if let Some(t) = signing_time {
        let civil = t.to_zoned(jiff::tz::TimeZone::UTC).datetime();
        let text = format!(
            "{:04}{:02}{:02}{:02}{:02}{:02}Z",
            civil.year(),
            civil.month(),
            civil.day(),
            civil.hour(),
            civil.minute(),
            civil.second()
        );
        let mut value = vec![
            0x18,
            u8::try_from(text.len()).expect("a time fits one length byte"),
        ];
        value.extend_from_slice(text.as_bytes());
        attrs.push(attr(db::rfc5911::ID_SIGNING_TIME, &value));
    }
    SetOfVec::try_from(attrs).expect("the attributes sort")
}

/// One `signing-certificate-v2` attribute for `cert`, with a `SHA-256` hash.
fn ess_signing_certificate_v2(cert: &Certificate) -> Attributes {
    ess_signing_certificate_v2_custom(cert, Prehash::Sha256, None, None)
}

/// One `signing-certificate-v2` attribute hashed with `hash`; `issuer` and
/// `serial`, when set, replace those of `cert`.
fn ess_signing_certificate_v2_custom(
    cert: &Certificate,
    hash: Prehash,
    issuer: Option<Name>,
    serial: Option<SerialNumber>,
) -> Attributes {
    let der = cert.to_der().expect("the certificate encodes");
    let hash_algorithm = match hash {
        Prehash::Sha256 => None,
        Prehash::Sha384 => Some(AlgorithmIdentifierOwned {
            oid: db::rfc5912::ID_SHA_384,
            parameters: None,
        }),
        Prehash::Sha512 => Some(AlgorithmIdentifierOwned {
            oid: db::rfc5912::ID_SHA_512,
            parameters: None,
        }),
    };
    let entry = EssSigningCertificate {
        hash_algorithm,
        hash: OctetString::new(hash.digest(&der)).expect("the hash encodes"),
        issuer: issuer.unwrap_or_else(|| cert.tbs_certificate.issuer.clone()),
        serial: serial.unwrap_or_else(|| cert.tbs_certificate.serial_number.clone()),
    };
    let entry_der = entry.to_der().expect("the entry encodes");
    SetOfVec::try_from(vec![attr(
        ESS_SIGNING_CERTIFICATE_V2,
        sequence_of(vec![entry_der]).as_slice(),
    )])
    .expect("a single value")
}

/// One `signing-certificate` attribute for `cert`, an `ESSCertID` with a
/// `SHA-1` hash and the `issuerSerial` of the referenced certificate.
fn ess_signing_certificate_v1(cert: &Certificate) -> Attributes {
    ess_signing_certificate_v1_custom(cert, None)
}

/// A `signing-certificate` attribute for `cert`; `serial`, when set,
/// replaces the referenced certificate's serial.
fn ess_signing_certificate_v1_custom(
    cert: &Certificate,
    serial: Option<SerialNumber>,
) -> Attributes {
    let der = cert.to_der().expect("the certificate encodes");
    let entry = EssSigningCertificateV1 {
        hash: OctetString::new(Sha1::digest(&der).to_vec()).expect("the hash encodes"),
        issuer_serial: Some(IssuerAndSerialNumber {
            issuer: cert.tbs_certificate.issuer.clone(),
            serial_number: serial.unwrap_or_else(|| cert.tbs_certificate.serial_number.clone()),
        }),
    };
    let entry_der = entry.to_der().expect("the entry encodes");
    SetOfVec::try_from(vec![attr(ESS_SIGNING_CERTIFICATE, &entry_der)]).expect("a single value")
}

/// The `CMS` in `der` with its signed attributes passed through `f`,
/// re-signed by the `ECDSA` key in `scalar` with `SHA-256`.
fn resign_cms(der: &[u8], scalar: &[u8], f: impl FnOnce(&mut Vec<Attribute>)) -> Vec<u8> {
    let content_info = ContentInfo::from_der(der).expect("valid cms");
    let mut signed_data =
        SignedData::from_der(&content_info.content.to_der().expect("der")).expect("valid cms");
    let mut signers: Vec<SignerInfo> = signed_data.signer_infos.0.into();
    let Some(signer) = signers.first_mut() else {
        panic!("a signer");
    };
    let mut attrs = signer
        .signed_attrs
        .as_ref()
        .expect("the attributes")
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    f(&mut attrs);
    *signer.signed_attrs.as_mut().expect("the attributes") =
        SetOfVec::try_from(attrs).expect("the attributes sort");
    let to_sign = signer
        .signed_attrs
        .as_ref()
        .expect("the attributes")
        .to_der()
        .expect("der");
    let key = SigningKey::<NistP256>::from_slice(scalar).expect("a valid scalar");
    let signature: ecdsa::Signature<NistP256> = key
        .sign_prehash(&Sha256::digest(&to_sign))
        .expect("the signature");
    signer.signature = OctetString::new(signature.to_vec()).expect("octets");
    signed_data.signer_infos = SignerInfos(SetOfVec::try_from(signers).expect("the signers sort"));
    let der = signed_data.to_der().expect("der");
    ContentInfo {
        content_type: db::rfc5911::ID_SIGNED_DATA,
        content: Any::from_der(&der).expect("valid DER"),
    }
    .to_der()
    .expect("der")
}

/// A `CMS SignedData` over `content`, signed by the first certificate of
/// `chain` with the `ECDSA` key in `scalar`; `extra_certs` are added to the
/// certificates set.
#[expect(
    clippy::too_many_arguments,
    reason = "the `SignedData` fields are flat"
)]
fn build_cms(
    content: &[u8],
    chain: Vec<Certificate>,
    signing_time: Option<Timestamp>,
    extra_signed: Option<Attributes>,
    extra_unsigned: Option<Attributes>,
    scalar: &[u8],
    algs: &Algs,
    extra_certs: Vec<Certificate>,
) -> Vec<u8> {
    let signer = &chain[0];
    let sid = IssuerAndSerialNumber {
        issuer: chain[1].tbs_certificate.subject.clone(),
        serial_number: signer.tbs_certificate.serial_number.clone(),
    };
    let key = SigningKey::<NistP256>::from_slice(scalar).expect("a valid scalar");
    let mut signed_attrs = signed_attrs(content, signing_time, algs.digest_hash());
    if let Some(extra) = extra_signed {
        signed_attrs
            .extend(extra.into_vec())
            .expect("the attributes sort");
    }
    let to_sign = signed_attrs.to_der().expect("the attributes encode");
    let signature: ecdsa::Signature<NistP256> = key
        .sign_prehash(&algs.prehash.digest(&to_sign))
        .expect("the signature");
    let digest_alg = AlgorithmIdentifierOwned {
        oid: algs.digest,
        parameters: None,
    };
    let signer_info = SignerInfo {
        version: CmsVersion::V1,
        sid: SignerIdentifier::IssuerAndSerialNumber(sid),
        digest_alg: digest_alg.clone(),
        signed_attrs: Some(signed_attrs),
        signature_algorithm: AlgorithmIdentifierOwned {
            oid: algs.signature,
            parameters: None,
        },
        // CMS carries the ECDSA signature as a DER `SEQUENCE { r, s }`, not raw `r || s`.
        signature: OctetString::new(signature.to_der().as_ref().to_vec())
            .expect("the signature encodes"),
        unsigned_attrs: extra_unsigned,
    };
    let mut certs: Vec<cms::cert::CertificateChoices> = chain
        .into_iter()
        .map(cms::cert::CertificateChoices::Certificate)
        .collect();
    for cert in extra_certs {
        certs.push(cms::cert::CertificateChoices::Certificate(cert));
    }
    let signed_data = SignedData {
        version: CmsVersion::V1,
        digest_algorithms: SetOfVec::try_from(vec![digest_alg]).expect("a single algorithm"),
        encap_content_info: EncapsulatedContentInfo {
            econtent_type: db::rfc5911::ID_DATA,
            econtent: Some(econtent_any(content)),
        },
        certificates: Some(CertificateSet(
            SetOfVec::try_from(certs).expect("the certificates sort"),
        )),
        crls: None,
        signer_infos: SignerInfos(SetOfVec::try_from(vec![signer_info]).expect("a single signer")),
    };
    let signed_data_der = signed_data.to_der().expect("the signed data encodes");
    let content_info = ContentInfo {
        content_type: db::rfc5911::ID_SIGNED_DATA,
        content: Any::from_der(signed_data_der.as_slice()).expect("valid DER"),
    };
    content_info.to_der().expect("the content info encodes")
}

/// The `eContent` `ANY` wrapping the raw `content` octets.
fn econtent_any(content: &[u8]) -> Any {
    let octets = OctetString::new(content.to_vec())
        .expect("the content encodes")
        .to_der()
        .expect("the content encodes");
    Any::from_der(octets.as_slice()).expect("valid DER")
}

/// One `signing-certificate-v2` entry.
#[derive(der::Sequence)]
struct EssSigningCertificate {
    /// The hash algorithm of `hash`, `SHA-256` when absent.
    hash_algorithm: Option<AlgorithmIdentifierOwned>,
    /// The hash of the certificate `DER` encoding.
    hash: OctetString,
    /// The issuer of the referenced certificate.
    issuer: Name,
    /// The serial number of the referenced certificate.
    serial: SerialNumber,
}

/// One `signing-certificate` entry, an `ESSCertID`.
#[derive(der::Sequence)]
struct EssSigningCertificateV1 {
    /// The hash of the certificate `DER` encoding.
    hash: OctetString,
    /// The issuer and serial of the referenced certificate.
    issuer_serial: Option<IssuerAndSerialNumber>,
}

/// `DER` and `base64`, one line and wrapped, all unwrap to the invoice.
#[test]
fn der_and_base64_both_unwrap() {
    let invoice = invoice();
    assert_eq!(
        unwrap(&support::fixture_bytes("invoice-rsa.p7m")).expect("valid DER"),
        invoice
    );
    assert_eq!(
        unwrap(&support::fixture_bytes("invoice-ec.p7m")).expect("valid DER"),
        invoice
    );
    for base64 in ["invoice-rsa.p7m.base64", "invoice-ec.p7m.base64"] {
        assert_eq!(
            unwrap(&support::fixture_bytes(base64)).expect("valid base64"),
            invoice,
            "{base64}"
        );
    }
    let flat = String::from_utf8(support::fixture_bytes("invoice-rsa.p7m.base64")).expect("ascii");
    let wrapped: String = flat
        .as_bytes()
        .chunks(76)
        .map(|chunk| std::str::from_utf8(chunk).expect("ascii"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(unwrap(wrapped.as_bytes()).expect("wrapped base64"), invoice);
}

/// A valid RSA `CAdES` signature verifies and exposes the content, the
/// signer, the chain and the signing time.
#[test]
fn valid_rsa_signature_verifies() {
    let invoice = invoice();
    let signer = cert("rsa-signer.pem");
    let verified = verify(
        &support::fixture_bytes("invoice-rsa.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect("the signature verifies");
    assert_eq!(verified.content(), invoice.as_slice());
    assert_eq!(verified.signer(), &signer);
    assert_eq!(verified.chain().len(), 3);
    let not_before = signer.tbs_certificate.validity.not_before.to_system_time();
    let not_after = signer.tbs_certificate.validity.not_after.to_system_time();
    let lo = not_before
        .duration_since(UNIX_EPOCH)
        .expect("after 1970")
        .as_secs();
    let hi = not_after
        .duration_since(UNIX_EPOCH)
        .expect("after 1970")
        .as_secs();
    let t = verified.signing_time().as_second();
    assert!(t >= i64::try_from(lo).expect("fits"));
    assert!(t <= i64::try_from(hi).expect("fits"));
    let doc = verified
        .document::<FatturaElettronica>()
        .expect("the content is an invoice");
    assert_eq!(
        doc,
        FatturaElettronica::from_xml_slice(&invoice).expect("the sample parses")
    );
}

/// A valid `ECDSA` `P-256` `CAdES` signature verifies.
#[test]
fn valid_ecdsa_signature_verifies() {
    let invoice = invoice();
    let verified = verify(
        &support::fixture_bytes("invoice-ec.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect("the signature verifies");
    assert_eq!(verified.content(), invoice.as_slice());
    assert_eq!(verified.signer(), &cert("ec-signer.pem"));
    assert_eq!(verified.chain().len(), 3);
}

/// A changed byte of the encapsulated content breaks the `message-digest`
/// check.
#[test]
fn tampered_content_fails_on_digest() {
    let mut der = support::fixture_bytes("invoice-rsa.p7m");
    let marker: &[u8] = b"<FatturaElettronica";
    let i = der
        .windows(marker.len())
        .position(|w| w == marker)
        .expect("the content is present");
    der[i + 1] ^= 0x40;
    let err = verify(&der, &[cert("ca.pem")], None).expect_err("the content was tampered with");
    assert!(matches!(err, SignedError::DigestMismatch), "{err:?}");
}

/// A changed byte of the signature breaks the signature check.
#[test]
fn tampered_signature_fails() {
    for fixture in ["invoice-rsa.p7m", "invoice-ec.p7m"] {
        let der = support::fixture_bytes(fixture);
        let content_info = ContentInfo::from_der(&der).expect("valid cms");
        let signed_data =
            SignedData::from_der(&content_info.content.to_der().expect("der")).expect("valid cms");
        let signer = signed_data.signer_infos.0.iter().next().expect("a signer");
        let signature = signer.signature.as_bytes();
        assert!(!signature.is_empty(), "a real signature");
        let start = der
            .windows(signature.len())
            .position(|w| w == signature)
            .expect("the signature is in the file");
        let mut tampered = der;
        tampered[start] ^= 0x40;
        let err = verify(&tampered, &[cert("ca.pem")], None)
            .expect_err("the signature was tampered with");
        assert!(
            matches!(err, SignedError::SignatureInvalid),
            "{fixture}: {err:?}"
        );
    }
}

/// An anchor that does not issue the chain is rejected.
#[test]
fn wrong_anchor_fails() {
    let err = verify(
        &support::fixture_bytes("invoice-rsa.p7m"),
        &[cert("other-root.pem")],
        None,
    )
    .expect_err("the anchor is wrong");
    assert!(
        matches!(err, SignedError::Chain(ChainError::IssuerNotInStore)),
        "{err:?}"
    );
}

/// A signer certificate expired at the signing time is rejected.
#[test]
fn expired_signer_fails() {
    let err = verify(
        &support::fixture_bytes("invoice-expired.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect_err("the signer is expired");
    assert!(
        matches!(err, SignedError::Chain(ChainError::Expired)),
        "{err:?}"
    );
}

/// A `P-521` signer uses a curve the verifier does not support.
#[test]
fn unsupported_curve_fails() {
    let err = verify(
        &support::fixture_bytes("invoice-p521.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect_err("the curve is unsupported");
    assert!(
        matches!(err, SignedError::UnsupportedAlgorithm(_)),
        "{err:?}"
    );
}

/// A detached `CMS` has no content to unwrap.
#[test]
fn detached_content_fails() {
    let p7m = support::fixture_bytes("invoice-detached.p7m");
    assert!(matches!(unwrap(&p7m), Err(SignedError::DetachedContent)));
    assert!(matches!(
        verify(&p7m, &[cert("ca.pem")], None),
        Err(SignedError::DetachedContent)
    ));
}

/// Inputs that are not `CMS` fail with a clear error.
#[test]
fn non_cms_input_fails() {
    let invoice = invoice();
    let b64 = base64::engine::general_purpose::STANDARD.encode(&invoice);
    let cert_der = cert("ca.pem").to_der().expect("the certificate encodes");
    for bad in [
        invoice.as_slice(),
        b64.as_bytes(),
        cert_der.as_slice(),
        b"not a cms at all",
    ] {
        assert!(
            matches!(unwrap(bad), Err(SignedError::NotCms(_))),
            "{bad:?}"
        );
    }
}

/// A `CMS` without a `signing-time` attribute is checked at the caller-
/// supplied time, and fails without one.
#[test]
fn no_signing_time_uses_caller_time() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let t = signing_time(&signer);
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        None,
        None,
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    let verified = verify(&p7m, &[cert("ca.pem")], Some(t)).expect("the caller time is used");
    assert_eq!(verified.signing_time(), &t);

    let early = t - SignedDuration::from_hours(2);
    let err = verify(&p7m, &[cert("ca.pem")], Some(early))
        .expect_err("before the certificates are valid");
    assert!(
        matches!(err, SignedError::Chain(ChainError::NotYetValid)),
        "{err:?}"
    );

    let err = verify(&p7m, &[cert("ca.pem")], None).expect_err("no time at all");
    assert!(matches!(err, SignedError::MissingSigningTime), "{err:?}");
}

/// A `signing-certificate-v2` attribute that references the signer
/// verifies.
#[test]
fn signing_certificate_v2_verifies() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let t = signing_time(&signer);
    let v2 = ess_signing_certificate_v2(&signer);
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        Some(v2),
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    verify(&p7m, &[cert("ca.pem")], Some(t)).expect("the v2 attribute matches the signer");
}

/// A `signing-certificate-v2` attribute that does not reference the signer
/// is rejected.
#[test]
fn signing_certificate_v2_mismatch_fails() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let t = signing_time(&signer);
    let other = cert("intermediate.pem");
    let v2 = ess_signing_certificate_v2(&other);
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        Some(v2),
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    let err = verify(&p7m, &[cert("ca.pem")], Some(t))
        .expect_err("the v2 attribute names the wrong certificate");
    assert!(
        matches!(err, SignedError::SigningCertificateMismatch),
        "{err:?}"
    );
}

/// A `CMS` without the `content-type` signed attribute is rejected.
#[test]
fn missing_content_type_fails() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let t = signing_time(&signer);
    let scalar = ec_signer_scalar();
    let der = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        None,
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    let p7m = resign_cms(&der, &scalar, |attrs| {
        attrs.retain(|attr| attr.oid != db::rfc5911::ID_CONTENT_TYPE);
    });
    let err = verify(&p7m, &[cert("ca.pem")], Some(t))
        .expect_err("the content-type attribute is missing");
    assert!(matches!(err, SignedError::ContentTypeMismatch), "{err:?}");
}

/// A `CMS` whose `content-type` signed attribute does not match the
/// encapsulated content is rejected.
#[test]
fn wrong_content_type_fails() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let t = signing_time(&signer);
    let scalar = ec_signer_scalar();
    let der = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        None,
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    let p7m = resign_cms(&der, &scalar, |attrs| {
        let attr = attrs
            .iter_mut()
            .find(|attr| attr.oid == db::rfc5911::ID_CONTENT_TYPE)
            .expect("the content-type attribute");
        attr.values = SetOfVec::try_from(vec![
            Any::from_der(
                db::rfc5911::ID_SIGNED_DATA
                    .to_der()
                    .expect("the OID encodes")
                    .as_slice(),
            )
            .expect("a valid OID"),
        ])
        .expect("a single value");
    });
    let err = verify(&p7m, &[cert("ca.pem")], Some(t))
        .expect_err("the content-type does not match the content");
    assert!(matches!(err, SignedError::ContentTypeMismatch), "{err:?}");
}

/// A `CMS` without the `message-digest` signed attribute is rejected.
#[test]
fn missing_message_digest_fails() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let t = signing_time(&signer);
    let scalar = ec_signer_scalar();
    let der = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        None,
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    let p7m = resign_cms(&der, &scalar, |attrs| {
        attrs.retain(|attr| attr.oid != db::rfc5911::ID_MESSAGE_DIGEST);
    });
    let err = verify(&p7m, &[cert("ca.pem")], Some(t))
        .expect_err("the message-digest attribute is missing");
    assert!(matches!(err, SignedError::DigestMismatch), "{err:?}");
}

/// A `signing-certificate-v2` attribute in the unsigned attributes is not
/// checked, so one that names the wrong certificate does not reject the
/// `CMS`.
#[test]
fn signing_certificate_v2_in_unsigned_attrs_is_ignored() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let t = signing_time(&signer);
    let other = cert("intermediate.pem");
    let v2 = ess_signing_certificate_v2(&other);
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        None,
        Some(v2),
        &scalar,
        &default_algs(),
        vec![],
    );
    verify(&p7m, &[cert("ca.pem")], Some(t)).expect("the unsigned attribute is not checked");
}

/// A `signing-certificate` attribute that references the signer verifies.
#[test]
fn signing_certificate_v1_verifies() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let t = signing_time(&signer);
    let v1 = ess_signing_certificate_v1(&signer);
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        Some(v1),
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    verify(&p7m, &[cert("ca.pem")], Some(t)).expect("the v1 attribute matches the signer");
}

/// A `signing-certificate` attribute that does not reference the signer is
/// rejected.
#[test]
fn signing_certificate_v1_mismatch_fails() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let t = signing_time(&signer);
    let other = cert("intermediate.pem");
    let v1 = ess_signing_certificate_v1(&other);
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        Some(v1),
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    let err = verify(&p7m, &[cert("ca.pem")], Some(t))
        .expect_err("the v1 attribute names the wrong certificate");
    assert!(
        matches!(err, SignedError::SigningCertificateMismatch),
        "{err:?}"
    );
}

/// A `digestAlgorithm` that does not agree with the hash named by the
/// signature algorithm is rejected.
#[test]
fn digest_algorithm_mismatch_fails() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let t = signing_time(&signer);
    let scalar = ec_signer_scalar();
    let algs = Algs {
        digest: db::rfc5912::ID_SHA_256,
        signature: db::rfc5912::ECDSA_WITH_SHA_384,
        prehash: Prehash::Sha384,
    };
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        None,
        None,
        &scalar,
        &algs,
        vec![],
    );
    let err = verify(&p7m, &[cert("ca.pem")], Some(t))
        .expect_err("the digest algorithm does not agree with the signature algorithm");
    assert!(matches!(err, SignedError::AlgorithmMismatch), "{err:?}");
}

/// A valid `RSA` `CAdES` signature made with `SHA-384` under
/// `rsaEncryption` verifies, with the hash taken from the
/// `digestAlgorithm`.
#[test]
fn valid_rsa_sha384_signature_verifies() {
    let invoice = invoice();
    let verified = verify(
        &support::fixture_bytes("invoice-rsa-sha384.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect("the signature verifies");
    assert_eq!(verified.content(), invoice.as_slice());
    assert_eq!(verified.signer(), &cert("rsa-signer.pem"));
    assert_eq!(verified.chain().len(), 3);
}

/// The `CMS` in `der` with the signature algorithm of the first signer
/// replaced by `oid`, everything else untouched.
fn rename_signature_oid(der: &[u8], oid: ObjectIdentifier) -> Vec<u8> {
    let content_info = ContentInfo::from_der(der).expect("valid cms");
    let mut signed_data =
        SignedData::from_der(&content_info.content.to_der().expect("der")).expect("valid cms");
    let mut signers: Vec<SignerInfo> = signed_data.signer_infos.0.into();
    signers
        .first_mut()
        .expect("a signer")
        .signature_algorithm
        .oid = oid;
    signed_data.signer_infos = SignerInfos(SetOfVec::try_from(signers).expect("the signers sort"));
    let der = signed_data.to_der().expect("der");
    ContentInfo {
        content_type: db::rfc5911::ID_SIGNED_DATA,
        content: Any::from_der(&der).expect("valid DER"),
    }
    .to_der()
    .expect("der")
}

/// The `CMS` in `der` with the certificate `cert` moved to the front of the
/// certificates set, the other members keeping their order.
fn move_cert_to_front(der: &[u8], cert: &Certificate) -> Vec<u8> {
    let content_info = ContentInfo::from_der(der).expect("valid cms");
    let signed_data =
        SignedData::from_der(&content_info.content.to_der().expect("der")).expect("valid cms");
    let set = &signed_data
        .certificates
        .as_ref()
        .expect("a certificates set");
    // The first certificate `TLV` in the file starts the set content.
    let first = set
        .0
        .iter()
        .map(|choice| match choice {
            cms::cert::CertificateChoices::Certificate(certificate) => {
                let tlv = certificate.to_der().expect("the certificate encodes");
                der.windows(tlv.len())
                    .position(|w| w == tlv.as_slice())
                    .expect("the certificate is in the file")
            }
            cms::cert::CertificateChoices::Other(_) => panic!("an other certificate format"),
        })
        .min()
        .expect("a certificate");
    let (header, start) = [4usize, 3, 2]
        .into_iter()
        .find_map(|h| {
            let start = first.checked_sub(h)?;
            (der[start] == 0xa0).then_some((h, start))
        })
        .expect("the certificates set header");
    let (len, _enc) = der_length(der, start + 1).expect("the set length");
    let end = start + header + len;
    let mut members: Vec<(usize, usize)> = Vec::new();
    let mut p = start + header;
    while p < end {
        assert_eq!(der[p], 0x30, "a certificate tag");
        let (mlen, menc) = der_length(der, p + 1).expect("a member length");
        members.push((p, p + 1 + menc + mlen));
        p += 1 + menc + mlen;
    }
    assert_eq!(p, end, "the set is fully parsed");
    let cert_tlv = cert.to_der().expect("the certificate encodes");
    let idx = members
        .iter()
        .position(|(s, e)| &der[*s..*e] == cert_tlv.as_slice())
        .expect("the member is the moved certificate");
    let mut content = Vec::with_capacity(len);
    for (i, (s, e)) in members.iter().enumerate() {
        if i == idx {
            content.extend_from_slice(&der[*s..*e]);
        }
    }
    for (i, (s, e)) in members.iter().enumerate() {
        if i != idx {
            content.extend_from_slice(&der[*s..*e]);
        }
    }
    let mut out = der.to_vec();
    out[start + header..end].copy_from_slice(&content);
    out
}

/// The `CMS` in `der` with its `eContent` wrapped in a constructed
/// `OCTET STRING`, the form the `.NET` signer writes.
fn construct_econtent(der: &[u8]) -> Vec<u8> {
    let id_data = db::rfc5911::ID_DATA.to_der().expect("the OID encodes");
    let oid = der
        .windows(id_data.len())
        .position(|w| w == id_data.as_slice())
        .expect("the idData OID");
    let signed_data_oid = db::rfc5911::ID_SIGNED_DATA
        .to_der()
        .expect("the OID encodes");
    let wrapper = der
        .windows(signed_data_oid.len())
        .position(|w| w == signed_data_oid.as_slice())
        .expect("the SignedData OID")
        + signed_data_oid.len();
    assert_eq!(der[wrapper], 0xa0, "the SignedData content tag");
    let (_, w_enc) = der_length(der, wrapper + 1).expect("the wrapper length");
    let signed_data = wrapper + 1 + w_enc;
    assert_eq!(der[signed_data], 0x30, "the SignedData tag");
    let after = oid + id_data.len();
    assert_eq!(der[after], 0xa0, "the encapContentInfo content tag");
    let (_, e_enc) = der_length(der, after + 1).expect("the content length");
    let octet = after + 1 + e_enc;
    assert_eq!(der[octet], 0x04, "the eContent tag");
    let (c_len, c_enc) = der_length(der, octet + 1).expect("the eContent length");
    let wrap_len = 1u32
        + u32::try_from(c_enc).expect("a length count")
        + u32::try_from(c_len).expect("a length");
    debug_assert!(wrap_len <= 0xffff, "the wrap fits the long form");
    let hi = u8::try_from(wrap_len >> 8).expect("the high byte of a `u16`");
    let lo = (wrap_len & 0xff) as u8; // the masked low byte of the length
    let mut out = der.to_vec();
    // The five enclosing long-form lengths gain the four wrapper bytes.
    let encap_seq = oid - 4;
    assert_eq!(out[encap_seq], 0x30, "the EncapsulatedContentInfo tag");
    for pos in [0usize, wrapper, signed_data, encap_seq, after] {
        assert_eq!(out[pos + 1], 0x82, "a long-form length");
        let value = (u32::from(out[pos + 2]) * 256 + u32::from(out[pos + 3]))
            .checked_add(4)
            .expect("a length");
        assert!(value <= 0xffff, "the length stays in the long form");
        out[pos + 2] = u8::try_from(value >> 8).expect("the high byte of a `u16`");
        out[pos + 3] = (value & 0xff) as u8; // the masked low byte of the length
    }
    out.splice(octet..octet, [0x24, 0x82, hi, lo]);
    out
}

/// A valid `RSA` `CAdES` signature made with `SHA-512` under
/// `rsaEncryption` verifies.
#[test]
fn valid_rsa_sha512_signature_verifies() {
    let invoice = invoice();
    let verified = verify(
        &support::fixture_bytes("invoice-rsa-sha512.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect("the signature verifies");
    assert_eq!(verified.content(), invoice.as_slice());
    assert_eq!(verified.signer(), &cert("rsa-signer.pem"));
    assert_eq!(verified.chain().len(), 3);
}

/// An `RSA` signature under the named-hash signature algorithm verifies,
/// one algorithm per row of the table.
#[test]
fn valid_rsa_named_hash_signatures_verify() {
    for (fixture, oid) in [
        ("invoice-rsa.p7m", db::rfc5912::SHA_256_WITH_RSA_ENCRYPTION),
        (
            "invoice-rsa-sha384.p7m",
            db::rfc5912::SHA_384_WITH_RSA_ENCRYPTION,
        ),
        (
            "invoice-rsa-sha512.p7m",
            db::rfc5912::SHA_512_WITH_RSA_ENCRYPTION,
        ),
    ] {
        let der = support::fixture_bytes(fixture);
        let patched = rename_signature_oid(&der, oid);
        assert_ne!(patched, der, "the signature algorithm is replaced");
        verify(&patched, &[cert("ca.pem")], None)
            .expect("the named hash agrees with the digest algorithm");
    }
}

/// A valid `ECDSA` `P-256` signature over `SHA-384` verifies.
#[test]
fn valid_ecdsa_p256_sha384_signature_verifies() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let t = signing_time(&signer);
    let scalar = ec_signer_scalar();
    let algs = Algs {
        digest: db::rfc5912::ID_SHA_384,
        signature: db::rfc5912::ECDSA_WITH_SHA_384,
        prehash: Prehash::Sha384,
    };
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        None,
        None,
        &scalar,
        &algs,
        vec![],
    );
    verify(&p7m, &[cert("ca.pem")], Some(t)).expect("the P-256 signature over SHA-384 verifies");
}

/// A valid `ECDSA` `P-384` signature verifies, under `SHA-384` and
/// `SHA-256`.
#[test]
fn valid_ecdsa_p384_signatures_verify() {
    let invoice = invoice();
    for fixture in ["invoice-ec-p384.p7m", "invoice-ec384-sha256.p7m"] {
        let verified = verify(&support::fixture_bytes(fixture), &[cert("ca.pem")], None)
            .expect("the P-384 signature verifies");
        assert_eq!(verified.content(), invoice.as_slice());
        assert_eq!(verified.signer(), &cert("ec384-signer.pem"));
    }
}

/// An intermediate signed by a key that does not match the store fails.
#[test]
fn forged_intermediate_fails() {
    let err = verify(
        &support::fixture_bytes("invoice-forged.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect_err("the link to the store is signed by the wrong key");
    assert!(
        matches!(err, SignedError::Chain(ChainError::InvalidSignature)),
        "{err:?}"
    );
}

/// An intermediate without the `CA` basic constraint cannot issue the
/// signer.
#[test]
fn intermediate_without_ca_constraint_fails() {
    let err = verify(
        &support::fixture_bytes("invoice-no-ca.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect_err("the intermediate is not a certificate authority");
    assert!(
        matches!(err, SignedError::Chain(ChainError::NotAca)),
        "{err:?}"
    );
}

/// A `pathLenConstraint` of the first intermediate is exceeded.
#[test]
fn path_length_violation_fails() {
    let err = verify(
        &support::fixture_bytes("invoice-pathlen.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect_err("the path length constraint is exceeded");
    assert!(
        matches!(err, SignedError::Chain(ChainError::PathLengthExceeded)),
        "{err:?}"
    );
}

/// A signer certificate not yet valid at the signing time fails before the
/// boundary second and passes at and after it.
#[test]
fn boundary_future_signer_checked_at_validity_edges() {
    let invoice = invoice();
    let signer = cert("boundary-future.pem");
    let scalar = ec_signer_scalar();
    let not_before = signer.tbs_certificate.validity.not_before.to_system_time();
    let t = Timestamp::from_second(
        i64::try_from(
            not_before
                .duration_since(UNIX_EPOCH)
                .expect("after 1970")
                .as_secs(),
        )
        .expect("fits"),
    )
    .expect("fits");
    for delta in [0, 1] {
        let time = t + SignedDuration::from_secs(delta);
        let p7m = build_cms(
            &invoice,
            vec![signer.clone(), cert("intermediate.pem"), cert("ca.pem")],
            Some(time),
            None,
            None,
            &scalar,
            &default_algs(),
            vec![],
        );
        verify(&p7m, &[cert("ca.pem")], Some(time)).expect("valid at and after the boundary");
    }
    for delta in [-2, -1] {
        let time = t + SignedDuration::from_secs(delta);
        let p7m = build_cms(
            &invoice,
            vec![signer.clone(), cert("intermediate.pem"), cert("ca.pem")],
            Some(time),
            None,
            None,
            &scalar,
            &default_algs(),
            vec![],
        );
        let err = verify(&p7m, &[cert("ca.pem")], Some(time)).expect_err("before the boundary");
        assert!(
            matches!(err, SignedError::Chain(ChainError::NotYetValid)),
            "{err:?}"
        );
    }
}

/// A signer certificate expired at the signing time fails after the
/// boundary second and passes at and before it.
#[test]
fn boundary_past_signer_checked_at_validity_edges() {
    let invoice = invoice();
    let signer = cert("boundary-past.pem");
    let scalar = ec_signer_scalar();
    let not_after = signer.tbs_certificate.validity.not_after.to_system_time();
    let t = Timestamp::from_second(
        i64::try_from(
            not_after
                .duration_since(UNIX_EPOCH)
                .expect("after 1970")
                .as_secs(),
        )
        .expect("fits"),
    )
    .expect("fits");
    for delta in [0, -1] {
        let time = t + SignedDuration::from_secs(delta);
        let p7m = build_cms(
            &invoice,
            vec![signer.clone(), cert("intermediate.pem"), cert("ca.pem")],
            Some(time),
            None,
            None,
            &scalar,
            &default_algs(),
            vec![],
        );
        verify(&p7m, &[cert("ca.pem")], Some(time)).expect("valid at and before the boundary");
    }
    for delta in [1, 2] {
        let time = t + SignedDuration::from_secs(delta);
        let p7m = build_cms(
            &invoice,
            vec![signer.clone(), cert("intermediate.pem"), cert("ca.pem")],
            Some(time),
            None,
            None,
            &scalar,
            &default_algs(),
            vec![],
        );
        let err = verify(&p7m, &[cert("ca.pem")], Some(time)).expect_err("after the boundary");
        assert!(
            matches!(err, SignedError::Chain(ChainError::Expired)),
            "{err:?}"
        );
    }
}

/// A signer whose extended key usage is `documentSigning` verifies.
#[test]
fn eku_document_signing_verifies() {
    let invoice = invoice();
    let verified = verify(
        &support::fixture_bytes("invoice-eku-docsigning.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect("document signing is an accepted usage");
    assert_eq!(verified.content(), invoice.as_slice());
    assert_eq!(verified.signer(), &cert("eku-docsigning.pem"));
}

/// A signer whose extended key usage is only `serverAuth` is rejected.
#[test]
fn eku_server_auth_only_fails() {
    let err = verify(
        &support::fixture_bytes("invoice-eku-serverauth.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect_err("server authentication is not a signing usage");
    assert!(
        matches!(err, SignedError::Chain(ChainError::ExtendedKeyUsage)),
        "{err:?}"
    );
}

/// A signer whose `keyUsage` extension permits only key encipherment is
/// rejected.
#[test]
fn key_usage_key_encipherment_only_fails() {
    let err = verify(
        &support::fixture_bytes("invoice-ku-keyenc.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect_err("key encipherment does not permit signing");
    assert!(matches!(err, SignedError::KeyUsage), "{err:?}");
}

/// A `CMS` whose certificates set lists a sibling leaf of the signer before
/// the signer still selects the signer by issuer and serial.
#[test]
fn sibling_leaf_listed_first_still_selects_the_signer() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let expected = signer.clone();
    let sibling = cert("sibling.pem");
    let t = signing_time(&signer);
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        None,
        None,
        &scalar,
        &default_algs(),
        vec![sibling.clone()],
    );
    let reordered = move_cert_to_front(&p7m, &sibling);
    let verified = verify(&reordered, &[cert("ca.pem")], Some(t))
        .expect("the signer is selected by issuer and serial");
    assert_eq!(verified.signer(), &expected);
}

/// A signer whose serial equals the serial of the intermediate certificate
/// still verifies, the intermediate not being excluded from the pool.
#[test]
fn serial_collision_does_not_exclude_the_intermediate() {
    let invoice = invoice();
    let signer = cert("collision.pem");
    let t = signing_time(&signer);
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        None,
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    verify(&p7m, &[cert("ca.pem")], Some(t))
        .expect("the intermediate is not excluded by the serial");
}

/// A `signing-certificate-v2` attribute with a `SHA-384` or `SHA-512`
/// `hashAlgorithm` that references the signer verifies.
#[test]
fn ess_v2_sha384_and_sha512_hashes_verify() {
    for hash in [Prehash::Sha384, Prehash::Sha512] {
        let invoice = invoice();
        let signer = cert("ec-signer.pem");
        let t = signing_time(&signer);
        let v2 = ess_signing_certificate_v2_custom(&signer, hash, None, None);
        let scalar = ec_signer_scalar();
        let p7m = build_cms(
            &invoice,
            vec![signer, cert("intermediate.pem"), cert("ca.pem")],
            Some(t),
            Some(v2),
            None,
            &scalar,
            &default_algs(),
            vec![],
        );
        verify(&p7m, &[cert("ca.pem")], Some(t)).expect("the v2 hash matches the signer");
    }
}

/// A `signing-certificate-v2` attribute with the signer hash but a wrong
/// `issuerSerial` is rejected.
#[test]
fn ess_v2_matching_hash_wrong_issuer_serial_fails() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let other = cert("intermediate.pem");
    let t = signing_time(&signer);
    let v2 = ess_signing_certificate_v2_custom(
        &signer,
        Prehash::Sha256,
        Some(other.tbs_certificate.issuer.clone()),
        Some(other.tbs_certificate.serial_number),
    );
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        Some(v2),
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    let err = verify(&p7m, &[cert("ca.pem")], Some(t))
        .expect_err("the issuerSerial does not match the signer");
    assert!(
        matches!(err, SignedError::SigningCertificateMismatch),
        "{err:?}"
    );
}

/// A `signing-certificate-v2` attribute with a wrong hash but the signer
/// `issuerSerial` is rejected.
#[test]
fn ess_v2_wrong_hash_matching_issuer_serial_fails() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let other = cert("intermediate.pem");
    let t = signing_time(&signer);
    let v2 = ess_signing_certificate_v2_custom(
        &other,
        Prehash::Sha256,
        Some(signer.tbs_certificate.issuer.clone()),
        Some(signer.tbs_certificate.serial_number.clone()),
    );
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        Some(v2),
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    let err =
        verify(&p7m, &[cert("ca.pem")], Some(t)).expect_err("the hash does not match the signer");
    assert!(
        matches!(err, SignedError::SigningCertificateMismatch),
        "{err:?}"
    );
}

/// A `signing-certificate` attribute with the signer hash and issuer but a
/// wrong serial is rejected.
#[test]
fn ess_v1_wrong_serial_fails() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let t = signing_time(&signer);
    let v1 = ess_signing_certificate_v1_custom(&signer, Some(SerialNumber::from(999u64)));
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        Some(v1),
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    let err =
        verify(&p7m, &[cert("ca.pem")], Some(t)).expect_err("the serial does not match the signer");
    assert!(
        matches!(err, SignedError::SigningCertificateMismatch),
        "{err:?}"
    );
}

/// A `CMS` whose `eContent` is a constructed `OCTET STRING`, the form the
/// `.NET` signer writes, unwraps and verifies.
#[test]
fn constructed_econtent_unwraps_and_verifies() {
    let invoice = invoice();
    let der = support::fixture_bytes("invoice-rsa.p7m");
    let constructed = construct_econtent(&der);
    assert_ne!(constructed, der, "the eContent is re-wrapped");
    assert_eq!(
        unwrap(&constructed).expect("the constructed eContent unwraps"),
        invoice
    );
    verify(&constructed, &[cert("ca.pem")], None).expect("the constructed eContent verifies");
}

/// A signer whose only extended key usage is the Microsoft `document signing`
/// usage verifies.
#[test]
fn eku_microsoft_document_signing_verifies() {
    let invoice = invoice();
    let verified = verify(
        &support::fixture_bytes("invoice-msdocsigning.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect("the Microsoft document signing usage permits signing");
    assert_eq!(verified.content(), invoice.as_slice());
    assert_eq!(verified.signer(), &cert("msdocsigning.pem"));
}

/// A signer whose key usage permits only digital signatures verifies.
#[test]
fn key_usage_digital_signature_only_verifies() {
    let invoice = invoice();
    let verified = verify(
        &support::fixture_bytes("invoice-ku-digsig.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect("the digital signature usage permits signing");
    assert_eq!(verified.content(), invoice.as_slice());
    assert_eq!(verified.signer(), &cert("ku-digsig.pem"));
}

/// An intermediate certificate that shares the signer's serial number still
/// completes the chain, the pool exclusion keeping it in the pool.
#[test]
fn intermediate_sharing_the_signers_serial_completes_the_chain() {
    let invoice = invoice();
    let verified = verify(
        &support::fixture_bytes("invoice-int7-signer7.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect("the intermediate is not excluded from the pool");
    assert_eq!(verified.content(), invoice.as_slice());
    assert_eq!(verified.chain().len(), 3);
}

/// A chain whose two intermediates share a subject still verifies, the lower
/// intermediate not being dropped from the pool by the exclusion.
#[test]
fn cross_subject_intermediates_keep_the_lower_in_the_pool() {
    let invoice = invoice();
    let verified = verify(
        &support::fixture_bytes("invoice-cross.p7m"),
        &[cert("ca.pem")],
        None,
    )
    .expect("the lower intermediate is kept in the pool");
    assert_eq!(verified.content(), invoice.as_slice());
    assert_eq!(verified.chain().len(), 4);
}

/// An `RSA` `SHA-512` signature recorded as `ECDSA-with-SHA512` does not
/// name a supported algorithm for an `RSA` key.
#[test]
fn rsa_signature_declared_as_ecdsa_is_unsupported() {
    let der = support::fixture_bytes("invoice-rsa-sha512.p7m");
    let renamed = rename_signature_oid(&der, db::rfc5912::ECDSA_WITH_SHA_512);
    let err = verify(&renamed, &[cert("ca.pem")], None)
        .expect_err("the algorithm is unsupported for the key");
    assert!(
        matches!(err, SignedError::UnsupportedAlgorithm(_)),
        "{err:?}"
    );
}

/// A `P-384` `SHA-256` signature recorded as `rsaEncryption` does not name
/// a supported algorithm.
#[test]
fn p384_sha256_signature_declared_as_rsaencryption_is_unsupported() {
    let der = support::fixture_bytes("invoice-ec384-sha256.p7m");
    let renamed = rename_signature_oid(&der, db::rfc5912::RSA_ENCRYPTION);
    let err = verify(&renamed, &[cert("ca.pem")], None)
        .expect_err("the algorithm is unsupported for the key");
    assert!(
        matches!(err, SignedError::UnsupportedAlgorithm(_)),
        "{err:?}"
    );
}

/// A `P-384` `SHA-384` signature recorded as `rsaEncryption` does not name
/// a supported algorithm.
#[test]
fn p384_sha384_signature_declared_as_rsaencryption_is_unsupported() {
    let der = support::fixture_bytes("invoice-ec-p384.p7m");
    let renamed = rename_signature_oid(&der, db::rfc5912::RSA_ENCRYPTION);
    let err = verify(&renamed, &[cert("ca.pem")], None)
        .expect_err("the algorithm is unsupported for the key");
    assert!(
        matches!(err, SignedError::UnsupportedAlgorithm(_)),
        "{err:?}"
    );
}

/// An `ECDSA` signature over `SHA-512` on a `P-384` key does not name a
/// supported algorithm, although the chain and the signature are otherwise
/// valid.
#[test]
fn ecdsa_sha512_signature_on_a_p384_key_is_unsupported() {
    let invoice = invoice();
    let signer = cert("ec384-signer.pem");
    let t = signing_time(&signer);
    let algs = Algs {
        digest: db::rfc5912::ID_SHA_512,
        signature: db::rfc5912::ECDSA_WITH_SHA_512,
        prehash: Prehash::Sha512,
    };
    let p7m = build_cms(
        &invoice,
        vec![signer.clone(), cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        Some(ess_signing_certificate_v2(&signer)),
        None,
        &ec_signer_scalar(),
        &algs,
        vec![],
    );
    let err = verify(&p7m, &[cert("ca.pem")], Some(t))
        .expect_err("the curve and hash name no supported algorithm");
    assert!(
        matches!(err, SignedError::UnsupportedAlgorithm(_)),
        "{err:?}"
    );
}

/// One `signing-certificate-v2` attribute for `cert`, with the given hash
/// algorithm and the given hash bytes.
fn ess_v2_with_hash_algorithm(
    cert: &Certificate,
    hash_algorithm: Option<AlgorithmIdentifierOwned>,
    hash: &[u8],
) -> Attributes {
    let entry = EssSigningCertificate {
        hash_algorithm,
        hash: OctetString::new(hash).expect("the hash encodes"),
        issuer: cert.tbs_certificate.issuer.clone(),
        serial: cert.tbs_certificate.serial_number.clone(),
    };
    let entry_der = entry.to_der().expect("the entry encodes");
    SetOfVec::try_from(vec![attr(
        ESS_SIGNING_CERTIFICATE_V2,
        sequence_of(vec![entry_der]).as_slice(),
    )])
    .expect("a single value")
}

/// A `signing-certificate-v2` entry that records a `SHA-256` hash algorithm
/// is honored for the certificate hash check.
#[test]
fn ess_v2_explicit_sha256_hash_algorithm_verifies() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let der = signer.to_der().expect("the certificate encodes");
    let v2 = ess_v2_with_hash_algorithm(
        &signer,
        Some(AlgorithmIdentifierOwned {
            oid: db::rfc5912::ID_SHA_256,
            parameters: None,
        }),
        &Sha256::digest(der.as_slice())[..],
    );
    let t = signing_time(&signer);
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        Some(v2),
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    verify(&p7m, &[cert("ca.pem")], Some(t))
        .expect("the explicit SHA-256 entry matches the signer");
}

/// A `signing-certificate-v2` entry that records a `SHA-1` hash algorithm is
/// honored for the certificate hash check.
#[test]
fn ess_v2_sha1_hash_algorithm_verifies() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let der = signer.to_der().expect("the certificate encodes");
    let v2 = ess_v2_with_hash_algorithm(
        &signer,
        Some(AlgorithmIdentifierOwned {
            oid: db::rfc5912::ID_SHA_1,
            parameters: None,
        }),
        &Sha1::digest(der.as_slice())[..],
    );
    let t = signing_time(&signer);
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        Some(v2),
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    verify(&p7m, &[cert("ca.pem")], Some(t)).expect("the SHA-1 entry matches the signer");
}

/// A `signing-certificate-v2` entry with an unsupported hash algorithm never
/// matches the signer, even when its hash is the `SHA-1` of the signer.
#[test]
fn ess_v2_unsupported_hash_algorithm_does_not_match() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let der = signer.to_der().expect("the certificate encodes");
    let v2 = ess_v2_with_hash_algorithm(
        &signer,
        Some(AlgorithmIdentifierOwned {
            oid: db::rfc5912::ID_SHA_224,
            parameters: None,
        }),
        &Sha1::digest(der.as_slice())[..],
    );
    let t = signing_time(&signer);
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        Some(v2),
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    let err =
        verify(&p7m, &[cert("ca.pem")], Some(t)).expect_err("the entry cannot match the signer");
    assert!(
        matches!(err, SignedError::SigningCertificateMismatch),
        "{err:?}"
    );
}

/// A `signing-certificate-v2` entry with an unsupported hash algorithm never
/// matches the signer, even when its hash is the `SHA-512` of the signer.
#[test]
fn ess_v2_unsupported_hash_algorithm_with_sha512_hash_does_not_match() {
    let invoice = invoice();
    let signer = cert("ec-signer.pem");
    let der = signer.to_der().expect("the certificate encodes");
    let v2 = ess_v2_with_hash_algorithm(
        &signer,
        Some(AlgorithmIdentifierOwned {
            oid: db::rfc5912::ID_SHA_224,
            parameters: None,
        }),
        &Sha512::digest(der.as_slice())[..],
    );
    let t = signing_time(&signer);
    let scalar = ec_signer_scalar();
    let p7m = build_cms(
        &invoice,
        vec![signer, cert("intermediate.pem"), cert("ca.pem")],
        Some(t),
        Some(v2),
        None,
        &scalar,
        &default_algs(),
        vec![],
    );
    let err =
        verify(&p7m, &[cert("ca.pem")], Some(t)).expect_err("the entry cannot match the signer");
    assert!(
        matches!(err, SignedError::SigningCertificateMismatch),
        "{err:?}"
    );
}
