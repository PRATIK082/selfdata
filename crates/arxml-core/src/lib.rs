//! Runtime support for ARXML documents that were generated from an AUTOSAR
//! XSD with [`xsd-parser`](https://crates.io/crates/xsd-parser).
//!
//! This crate contains **no** schema knowledge and **no** dependency on
//! `xsd-parser` itself. It only talks to the traits that the generated code
//! implements:
//!
//! * [`xsd_parser_types::quick_xml::WithDeserializer`] – strict, streaming reader
//! * [`xsd_parser_types::quick_xml::WithSerializer`] – writer
//!
//! That makes the generated model crates usable as a standalone base library:
//! only `xsd-parser-types`, `quick-xml` and `regex` are needed at runtime.
//!
//! # Example
//!
//! ```no_run
//! use std::io::Cursor;
//! use arxml_core::{read_arxml, to_xml_string, validate_arxml};
//! # use xsd_parser_types::quick_xml::{WithDeserializer, WithSerializer};
//! # fn demo<T: WithDeserializer + WithSerializer>(bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
//!
//! let model: T = read_arxml(Cursor::new(bytes))?;
//! let xml = to_xml_string(&model, "AUTOSAR")?;
//! validate_arxml(&model, "AUTOSAR")?;
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::io::{BufRead, Write};

pub mod dom;

use xsd_parser_types::quick_xml::reader::{ErrorReader, IoReader, SliceReader, XmlReader, XmlReaderSync};
use xsd_parser_types::quick_xml::{
    BytesDecl, DeserializeSync, Error, Event, SerializeSync, WithDeserializer, WithSerializer,
    Writer,
};

/// Namespace of the AUTOSAR R4.0 schema family.
pub const AUTOSAR_NS: &str = "http://autosar.org/schema/r4.0";

/// Name of the root element of every AUTOSAR document.
pub const AUTOSAR_ROOT: &str = "AUTOSAR";

/// Error type used by all fallible operations of this crate.
#[derive(Debug)]
pub enum ArxmlError {
    /// The underlying reader/writer failed.
    Io(std::io::Error),
    /// The document is not valid XML or does not match the schema.
    Xml(Error),
    /// The document is schema-valid but semantically rejected.
    Validation(String),
    /// The document targets an unsupported AUTOSAR version/namespace.
    UnsupportedVersion(String),
}

impl fmt::Display for ArxmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Xml(err) => write!(f, "XML error: {err}"),
            Self::Validation(msg) => write!(f, "validation error: {msg}"),
            Self::UnsupportedVersion(msg) => write!(f, "unsupported version: {msg}"),
        }
    }
}

impl std::error::Error for ArxmlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Xml(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ArxmlError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<Error> for ArxmlError {
    fn from(value: Error) -> Self {
        Self::Xml(value)
    }
}

/// Read an ARXML document from a stream.
///
/// The reader is event driven: the document is never loaded into memory as a
/// whole, only one XML event at a time is materialized.
///
/// `T` is the root type generated for the concrete AUTOSAR release, e.g.
/// `arxml_model_r4_0::AutosarType`.
pub fn read_arxml<R, T>(reader: R) -> Result<T, ArxmlError>
where
    R: BufRead,
    T: WithDeserializer,
{
    let mut reader: ErrorReader<IoReader<R>> = IoReader::new(reader).with_error_info();
    Ok(<T as DeserializeSync<'static, ErrorReader<IoReader<R>>>>::deserialize(
        &mut reader,
    )?)
}

/// Read an ARXML document from a byte slice.
pub fn read_arxml_from_slice<T>(data: &[u8]) -> Result<T, ArxmlError>
where
    T: WithDeserializer,
{
    read_arxml(data)
}

/// Write an ARXML document to a writer.
///
/// The XML declaration is written by this function; the root element (and its
/// `xmlns` declaration) is written by the generated serializer.
pub fn write_arxml<W, T>(model: &T, writer: W, root_name: &str) -> Result<(), ArxmlError>
where
    W: Write,
    T: WithSerializer,
{
    let mut writer = Writer::new(writer);
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(ArxmlError::Io)?;
    model.serialize(root_name, &mut writer)?;
    writer.into_inner().flush().map_err(ArxmlError::Io)
}

/// Serialize an ARXML document into a `String`.
pub fn to_xml_string<T>(model: &T, root_name: &str) -> Result<String, ArxmlError>
where
    T: WithSerializer,
{
    let mut buffer = Vec::new();
    write_arxml(model, &mut buffer, root_name)?;
    Ok(String::from_utf8(buffer).map_err(|err| {
        ArxmlError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            err.to_string(),
        ))
    })?)
}

/// Validate an ARXML model.
///
/// The model is serialized and parsed back with the *strict* generated
/// deserializer. This checks required elements, element cardinality,
/// enumerations, simple-type facets (pattern, length, …), duplicate and
/// unexpected elements as well as unexpected attributes – i.e. everything the
/// XSD expresses.
///
/// Returns `Ok(())` when the model is valid, otherwise a human readable
/// description of the first violation.
pub fn validate_arxml<T>(model: &T, root_name: &str) -> Result<(), String>
where
    T: WithSerializer + WithDeserializer,
{
    let xml = to_xml_string(model, root_name).map_err(|err| err.to_string())?;
    let bytes = xml.as_bytes();
    let mut reader: ErrorReader<IoReader<&[u8]>> = IoReader::new(bytes).with_error_info();
    <T as DeserializeSync<'static, ErrorReader<IoReader<&[u8]>>>>::deserialize(&mut reader)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

/// Return the namespace of the root element of an ARXML document.
///
/// Useful to dispatch a document to the model of the matching AUTOSAR release.
pub fn detect_namespace(data: &[u8]) -> Option<String> {
    let mut reader = SliceReader::from_bytes(data);
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                for attrib in element.attributes().flatten() {
                    if attrib.key.as_ref() == b"xmlns" {
                        return Some(String::from_utf8_lossy(attrib.value.as_ref()).into_owned());
                    }
                }
                return None;
            }
            Ok(_) => continue,
            Err(_) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_root_namespace() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<AUTOSAR xmlns="http://autosar.org/schema/r4.0"><AR-PACKAGES/></AUTOSAR>"#;
        assert_eq!(
            detect_namespace(xml).as_deref(),
            Some("http://autosar.org/schema/r4.0")
        );
    }

    #[test]
    fn detects_missing_namespace() {
        let xml = br#"<AUTOSAR/>"#;
        assert_eq!(detect_namespace(xml), None);
    }
}
