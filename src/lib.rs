//! Reading and writing of every `FatturaPA` invoice and SDI message.
#![doc = include_str!("../README.md")]

pub mod amount;
pub mod date;
pub mod error;
pub mod filename;
pub mod generated;
pub mod ids;
pub mod invoice;
pub mod party;
pub mod signed;
#[cfg(feature = "trust-list")]
pub mod trust_list;
pub mod validate;

use std::borrow::Cow;
use std::io::{Read, Write};

use encoding_rs::Encoding;
use quick_xml::Reader;
use quick_xml::events::{BytesDecl, Event};
use xsd_parser_types::quick_xml::{DeserializeSync, Error as XmlError, SerializeSync, SliceReader};

pub use amount::Amount;
pub use date::{Date, DateTime};
pub use error::Error;
pub use filename::{InvoiceName, MessageKind, MessageName, NameError};
pub use generated::fpr12::FatturaElettronicaType as FatturaElettronica;
pub use generated::fsm10::FatturaElettronicaType as FatturaElettronicaSemplificata;
pub use generated::sdi::{
    AttestazioneTrasmissioneFatturaType as AttestazioneTrasmissioneFattura,
    MetadatiInvioFileType as MetadatiInvioFile,
    NotificaDecorrenzaTerminiType as NotificaDecorrenzaTermini,
    NotificaEsitoCommittenteType as NotificaEsitoCommittente, NotificaEsitoType as NotificaEsito,
    NotificaMancataConsegnaType as NotificaMancataConsegna, NotificaScartoType as NotificaScarto,
    RicevutaConsegnaType as RicevutaConsegna, ScartoEsitoCommittenteType as ScartoEsitoCommittente,
};
pub use invoice::{
    BuildError, Invoice, InvoiceCurrency, Line, Recipient, Sconto, SelfInvoice, SevenTerLine,
    VatTreatment,
};
pub use party::{Address, Business, Buyer, Company, Party, Seller, TaxIdentity};
pub use validate::Violation;

/// A `FatturaPA` invoice or SDI message that reads from XML and writes back
/// to XML.
///
/// Every root type implements the trait, with the prefixed root element name
/// carried in `ROOT`. The serialised form is preceded by the XML declaration
/// and the root element is written with its prefixed name, the form the
/// official schemas and samples use.
pub trait Document:
    SerializeSync<Error = XmlError> + for<'de> DeserializeSync<'de, SliceReader<'de>, Error = XmlError>
{
    /// The prefixed root element name of the document.
    const ROOT: &'static str;

    /// Reads the document from `input`, decoding the `encoding` the XML
    /// declaration names, or `UTF-8` when it names none.
    ///
    /// # Errors
    ///
    /// Fails with [`Error`] when the input cannot be read to the end, when
    /// the declared `encoding` is unsupported or the bytes do not decode, or
    /// when the input is not a well-formed document of this type.
    fn from_reader(input: impl Read) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let buffer = read_to_end_buffer(input)?;
        Self::from_xml_slice(&buffer)
    }

    /// Reads the document from the UTF-8 text in `input`.
    ///
    /// # Errors
    ///
    /// Fails with [`Error`] when `input` is not a well-formed document of
    /// this type.
    fn from_xml_str(input: &str) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let mut reader = SliceReader::new(input);
        Ok(DeserializeSync::deserialize(&mut reader)?)
    }

    /// Reads the document from the bytes in `input`, decoding the `encoding`
    /// the XML declaration names, or `UTF-8` when it names none.
    ///
    /// # Errors
    ///
    /// Fails with [`Error`] when the declared `encoding` is unsupported or
    /// the bytes do not decode, or when `input` is not a well-formed document
    /// of this type.
    fn from_xml_slice(input: &[u8]) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let utf8 = to_utf8(input)?;
        let mut reader = SliceReader::from_bytes(&utf8);
        Ok(DeserializeSync::deserialize(&mut reader)?)
    }

    /// Writes the document to `output`, preceded by the XML declaration.
    ///
    /// # Errors
    ///
    /// Fails with [`Error`] when the document cannot be serialized or the
    /// output cannot be written.
    fn to_writer(&self, output: &mut impl Write) -> Result<(), Error> {
        let mut writer = quick_xml::Writer::new(output);
        writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
        self.serialize(Self::ROOT, &mut writer)?;
        Ok(())
    }

    /// Writes the document to a UTF-8 string, preceded by the XML declaration.
    ///
    /// # Errors
    ///
    /// Fails with [`Error`] when the document cannot be serialized.
    fn to_xml_string(&self) -> Result<String, Error> {
        let bytes = self.to_xml_vec()?;
        Ok(String::from_utf8(bytes)?)
    }

    /// Writes the document to UTF-8 bytes, preceded by the XML declaration.
    ///
    /// # Errors
    ///
    /// Fails with [`Error`] when the document cannot be serialized.
    fn to_xml_vec(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        self.to_writer(&mut bytes)?;
        Ok(bytes)
    }
}

impl Document for FatturaElettronica {
    /// The root element name of the ordinary invoice.
    const ROOT: &'static str = "p:FatturaElettronica";
}

impl Document for FatturaElettronicaSemplificata {
    /// The root element name of the simplified invoice.
    const ROOT: &'static str = "p:FatturaElettronicaSemplificata";
}

impl Document for AttestazioneTrasmissioneFattura {
    /// The root element name of the `AT` message.
    const ROOT: &'static str = "types:AttestazioneTrasmissioneFattura";
}

impl Document for MetadatiInvioFile {
    /// The root element name of the `MT` message.
    const ROOT: &'static str = "types:MetadatiInvioFile";
}

impl Document for NotificaDecorrenzaTermini {
    /// The root element name of the `DT` message.
    const ROOT: &'static str = "types:NotificaDecorrenzaTermini";
}

impl Document for NotificaEsitoCommittente {
    /// The root element name of the `EC` message.
    const ROOT: &'static str = "types:NotificaEsitoCommittente";
}

impl Document for NotificaEsito {
    /// The root element name of the `NE` message.
    const ROOT: &'static str = "types:NotificaEsito";
}

impl Document for NotificaMancataConsegna {
    /// The root element name of the `MC` message.
    const ROOT: &'static str = "types:NotificaMancataConsegna";
}

impl Document for NotificaScarto {
    /// The root element name of the `NS` message.
    const ROOT: &'static str = "types:NotificaScarto";
}

impl Document for RicevutaConsegna {
    /// The root element name of the `RC` message.
    const ROOT: &'static str = "types:RicevutaConsegna";
}

impl Document for ScartoEsitoCommittente {
    /// The root element name of the `SE` message.
    const ROOT: &'static str = "types:ScartoEsitoCommittente";
}

/// The `encoding` the XML declaration of `input` names, resolved to the
/// `encoding_rs` encoding it names, or `UTF-8` when the declaration is absent
/// or names no `encoding`.
///
/// # Errors
///
/// Fails with [`Error::QuickXml`] when the first event cannot be read, and
/// with [`Error::UnsupportedEncoding`] when the declaration names an
/// `encoding` the `WHATWG` labels do not resolve.
fn declared_encoding(input: &[u8]) -> Result<&'static Encoding, Error> {
    // The declaration, when present, is the first event, and its bytes are
    // `ASCII`, so it reads regardless of the body encoding. A document opening
    // without a declaration uses the `UTF-8` default.
    let mut reader = Reader::from_reader(input);
    let mut buffer = Vec::new();
    let Event::Decl(decl) = reader.read_event_into(&mut buffer)? else {
        return Ok(encoding_rs::UTF_8);
    };
    // A declaration resolving to an `encoding` decodes in it.
    let Some(encoding) = decl.encoder() else {
        // An `encoding` the `WHATWG` labels do not resolve is rejected, naming
        // it. A declaration without an `encoding` keeps the `UTF-8` default.
        let name = decl.encoding().and_then(Result::ok).map(Cow::into_owned);
        return name.map_or_else(
            || Ok(encoding_rs::UTF_8),
            |name| Err(Error::UnsupportedEncoding(name)),
        );
    };
    Ok(encoding)
}

/// Decodes the bytes of `input` to `UTF-8`.
///
/// A leading byte order mark decides the `encoding` and is stripped before
/// decoding. Without a BOM the `encoding` the XML declaration names is used,
/// the `UTF-16` family reading its bytes as `UTF-8` per its `WHATWG` output
/// encoding. A `UTF-8` document passes through borrowed and every other
/// supported `encoding` is transcoded to fresh bytes.
///
/// # Errors
///
/// Fails with [`Error`] when the declared `encoding` is unsupported or the
/// bytes do not decode as it.
fn to_utf8(input: &[u8]) -> Result<Cow<'_, [u8]>, Error> {
    // A byte order mark decides the `encoding` on its own, so the declaration
    // is ignored and the mark is stripped before decoding.
    if let Some((encoding, bom_len)) = Encoding::for_bom(input) {
        let body = &input[bom_len..];
        if encoding == encoding_rs::UTF_8 {
            return Ok(Cow::Borrowed(body));
        }
        let decoded = quick_xml::encoding::decode(body, encoding)?;
        return Ok(Cow::Owned(decoded.into_owned().into_bytes()));
    }
    // A BOM-less `UTF-16` declaration names bytes that are not actually
    // `UTF-16`, so the resolved `encoding` is lowered by its `WHATWG` output
    // encoding, which the `UTF-16` family maps to `UTF-8`.
    let encoding = declared_encoding(input)?.output_encoding();
    if encoding == encoding_rs::UTF_8 {
        return Ok(Cow::Borrowed(input));
    }
    let decoded = quick_xml::encoding::decode(input, encoding)?;
    Ok(Cow::Owned(decoded.into_owned().into_bytes()))
}

/// Reads `input` to the end into a fresh buffer.
fn read_to_end_buffer(input: impl Read) -> Result<Vec<u8>, Error> {
    let mut input = input;
    let mut buffer = Vec::new();
    input.read_to_end(&mut buffer)?;
    Ok(buffer)
}
