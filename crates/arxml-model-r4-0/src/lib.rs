//! AUTOSAR R4.0 model generated from `schemas/AUTOSAR_00047.xsd`.
//!
//! The module [`model`] contains the generated types of the AUTOSAR namespace
//! (the root element is [`Autosar`]/[`AutosarType`]). The generated code only
//! depends on `xsd-parser-types`, `quick-xml` and `regex` – the `xsd-parser`
//! code generator itself is *not* needed at runtime.
//!
//! The helpers in [`api`] are thin wrappers around [`arxml_core`] that fix the
//! root element name (`AUTOSAR`) and the target namespace of this release.
//!
//! ```no_run
//! use arxml_model_r4_0::api;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let file = std::fs::File::open("model.arxml")?;
//! let model = api::read(std::io::BufReader::new(file))?;
//! api::validate(&model)?;
//! let xml = api::to_string(&model)?;
//! std::fs::write("model.out.arxml", xml)?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "generated")]
pub mod generated;

#[cfg(feature = "generated")]
pub use generated::ar as model;

#[cfg(feature = "generated")]
pub use generated::ar::{Autosar, AutosarType};

/// Namespace of the AUTOSAR release this model was generated from.
pub const AUTOSAR_NS: &str = "http://autosar.org/schema/r4.0";

/// Name of the root element of an AUTOSAR document.
pub const ROOT_ELEMENT: &str = "AUTOSAR";

/// Version specific read / write / validate helpers.
#[cfg(feature = "generated")]
pub mod api {
    use std::io::{BufRead, Write};

    use arxml_core::{self, ArxmlError};

    use super::{AutosarType, ROOT_ELEMENT};

    /// Read an AUTOSAR R4.0 document from a stream.
    pub fn read<R: BufRead>(reader: R) -> Result<AutosarType, ArxmlError> {
        arxml_core::read_arxml(reader)
    }

    /// Read an AUTOSAR R4.0 document from a byte slice.
    pub fn read_from_slice(data: &[u8]) -> Result<AutosarType, ArxmlError> {
        arxml_core::read_arxml_from_slice(data)
    }

    /// Write an AUTOSAR R4.0 document to a writer.
    pub fn write<W: Write>(model: &AutosarType, writer: W) -> Result<(), ArxmlError> {
        arxml_core::write_arxml(model, writer, ROOT_ELEMENT)
    }

    /// Serialize an AUTOSAR R4.0 document into a `String`.
    pub fn to_string(model: &AutosarType) -> Result<String, ArxmlError> {
        arxml_core::to_xml_string(model, ROOT_ELEMENT)
    }

    /// Validate an AUTOSAR R4.0 model against the schema.
    pub fn validate(model: &AutosarType) -> Result<(), String> {
        arxml_core::validate_arxml(model, ROOT_ELEMENT)
    }
}
