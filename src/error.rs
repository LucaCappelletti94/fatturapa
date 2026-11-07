//! The error type returned by every read and write function.

/// An error while reading or writing a `FatturaPA` document.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The XML is malformed or does not match the structure of the document.
    #[error(transparent)]
    Xml(#[from] xsd_parser_types::quick_xml::Error),

    /// A `quick_xml` failure while parsing the declaration or serialising the
    /// document.
    #[error(transparent)]
    QuickXml(#[from] quick_xml::Error),

    /// An I/O operation on the input or the output failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// The serialized document was not valid UTF-8.
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),

    /// The XML declaration names an `encoding` the reader cannot decode.
    #[error("the XML declares the unsupported encoding {0}")]
    UnsupportedEncoding(String),

    /// The document bytes do not decode as the `encoding` the declaration names.
    #[error(transparent)]
    Undecodable(#[from] quick_xml::encoding::EncodingError),
}
