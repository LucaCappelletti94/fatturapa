//! The error type returned by every read and write function.

/// An error while reading or writing a `FatturaPA` document.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The XML is malformed or does not match the structure of the document.
    #[error(transparent)]
    Xml(#[from] xsd_parser_types::quick_xml::Error),

    /// The XML could not be serialized.
    #[error(transparent)]
    XmlWriter(#[from] quick_xml::Error),

    /// An I/O operation on the input or the output failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// The serialized document was not valid UTF-8.
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
}
