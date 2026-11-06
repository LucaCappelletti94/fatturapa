//! Trust anchors from an ETSI trusted services list (TSL).
//!
//! The input is the `XML` of a trusted list as bytes, for example the
//! Italian list fetched over `HTTPS` from
//! `https://eidas.agid.gov.it/TL/TSL-IT.xml`. No network access is performed
//! in this crate, and the `XML` signature of the trusted list itself is not
//! verified. Fetching the list over `HTTPS` from the issuing authority and
//! validating the list's own signature are the caller's responsibility.
//!
//! The list format is the ETSI TS 102 231 style used by the current `AgID`
//! list. The root element is `TrustServiceStatusList`, and each service is a
//! `TSPService` whose `ServiceInformation` carries a `ServiceTypeIdentifier`
//! URN, a `ServiceStatus` and the certificate. The `ServiceHistory` of a
//! service records its past statuses and is not read.

use std::io::Cursor;

use base64::Engine;
use der::Decode;
use quick_xml::Reader;
use quick_xml::events::Event;
use x509_cert::Certificate;

/// The URN of the qualified certificate authority for certificates service
/// type.
const QC_CA_URN: &[u8] = b"http://uri.etsi.org/TrstSvc/Svctype/CA/QC";

/// An error while parsing a trusted services list.
#[derive(Debug, thiserror::Error)]
pub enum TrustListError {
    /// The list is not well-formed `XML`.
    #[error("the trusted list is not well-formed XML: {0}")]
    Xml(quick_xml::errors::Error),

    /// A certificate entry is not valid `base64`.
    #[error("a certificate in the trusted list is not valid base64: {0}")]
    Base64(base64::DecodeError),

    /// A certificate entry is not a valid `X.509` certificate.
    #[error("a certificate in the trusted list is not a valid certificate: {0}")]
    Der(der::Error),

    /// The `XML` is not a trusted services list.
    #[error("the XML is not a trusted services list")]
    NotTrustedList,
}

/// Extracts the trust anchors from a trusted services list.
///
/// A trust anchor is the certificate of a service whose type is a qualified
/// certificate authority for certificates and whose status is `granted`, in
/// the order the services appear in the list. Certificates of services of
/// other types and of the `PointersToOtherTSL` section are not returned.
///
/// # Errors
///
/// Fails with [`TrustListError`] when the list is not well-formed `XML`, is
/// not a trusted services list, or carries a malformed certificate entry.
pub fn trust_anchors(tsl: &[u8]) -> Result<Vec<Certificate>, TrustListError> {
    let mut reader = Reader::from_reader(Cursor::new(tsl));
    let mut buffer = Vec::new();
    let mut text = Vec::new();
    let mut cert_text = String::new();
    let mut anchors = Vec::new();
    let mut first = true;
    let mut root_closed = false;
    let mut in_type = false;
    let mut in_status = false;
    let mut in_cert = false;
    let mut in_history = false;
    // `ServiceHistory` repeats the type and status elements with past values,
    // so this state is scoped to the `ServiceInformation` of the current `TSPService`.
    let mut service_type: Option<Vec<u8>> = None;
    let mut status_granted = false;
    let mut service_cert: Option<String> = None;
    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(TrustListError::Xml)?
        {
            Event::Start(tag) => {
                let name = tag.local_name().into_inner();
                if first {
                    first = false;
                    if name != "TrustServiceStatusList" {
                        return Err(TrustListError::NotTrustedList);
                    }
                }
                match name {
                    "ServiceTypeIdentifier" => {
                        in_type = true;
                        text.clear();
                    }
                    "ServiceStatus" => {
                        in_status = true;
                        text.clear();
                    }
                    "X509Certificate" => {
                        in_cert = true;
                        cert_text.clear();
                    }
                    "ServiceHistory" => in_history = true,
                    _ => {}
                }
            }
            Event::Text(event) => {
                let content = event.into_inner();
                if in_type || in_status {
                    text.extend_from_slice(content.as_bytes());
                } else if in_cert {
                    cert_text.push_str(&content);
                }
            }
            Event::End(tag) => {
                let name = tag.local_name().into_inner();
                match name {
                    "ServiceTypeIdentifier" => {
                        in_type = false;
                        if !in_history && service_type.is_none() {
                            service_type = Some(text.trim_ascii().to_vec());
                        }
                    }
                    "ServiceStatus" => {
                        in_status = false;
                        if !in_history && !status_granted {
                            // The current list encodes the status as the URN `...Svcstatus/granted`.
                            status_granted = text.trim_ascii().ends_with(b"granted");
                        }
                    }
                    "X509Certificate" => {
                        in_cert = false;
                        if !in_history && service_cert.is_none() {
                            service_cert = Some(std::mem::take(&mut cert_text));
                        }
                    }
                    "ServiceHistory" => in_history = false,
                    "TSPService" => {
                        if service_type.as_deref() == Some(QC_CA_URN)
                            && status_granted
                            && let Some(cert) = service_cert.take()
                        {
                            anchors.push(parse_certificate(&cert)?);
                        }
                        service_type = None;
                        status_granted = false;
                        service_cert = None;
                    }
                    "TrustServiceStatusList" => root_closed = true,
                    _ => {}
                }
            }
            Event::Eof => {
                // An unclosed root element is a truncated list, which must fail.
                if !root_closed {
                    return Err(TrustListError::NotTrustedList);
                }
                break;
            }
            _ => {}
        }
    }
    Ok(anchors)
}

/// One `X509Certificate` entry of the list, `base64` of the certificate
/// `DER` encoding.
fn parse_certificate(text: &str) -> Result<Certificate, TrustListError> {
    let stripped: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    let der = base64::engine::general_purpose::STANDARD
        .decode(&stripped)
        .map_err(TrustListError::Base64)?;
    Certificate::from_der(&der).map_err(TrustListError::Der)
}
