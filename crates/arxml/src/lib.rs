//! Version aware facade for AUTOSAR (ARXML) documents.
//!
//! Every AUTOSAR release ships its own generated model crate
//! (`arxml-model-r4-0`, …). This crate hides that detail:
//!
//! * [`ArxmlVersion`] lists the supported releases and their namespaces,
//! * [`detect_version`] finds the release of a document by looking at the
//!   namespace of the root element,
//! * [`Model`] is a version independent container that can be read, written
//!   and validated.
//!
//! ```no_run
//! use arxml::{detect_version, Model};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let data = std::fs::read("model.arxml")?;
//! println!("detected: {:?}", detect_version(&data));
//! let model = Model::read(&data)?;
//! model.validate()?;
//! # Ok(())
//! # }
//! ```

use std::io::Write;

pub use arxml_core::{
    detect_namespace, dom, read_arxml, read_arxml_from_slice, to_xml_string, validate_arxml,
    write_arxml, ArxmlError, AUTOSAR_NS, AUTOSAR_ROOT,
};

/// A supported AUTOSAR schema release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ArxmlVersion {
    /// AUTOSAR R4.0 – schema `AUTOSAR_00047.xsd`.
    R4_0,
}

impl ArxmlVersion {
    /// All supported releases.
    pub const ALL: &'static [Self] = &[Self::R4_0];

    /// The XML namespace used by the root element of this release.
    pub fn namespace(&self) -> &'static str {
        match self {
            Self::R4_0 => "http://autosar.org/schema/r4.0",
        }
    }

    /// Human readable label of the release.
    pub fn label(&self) -> &'static str {
        match self {
            Self::R4_0 => "R4.0",
        }
    }

    /// Resolve a release from the namespace of a document.
    pub fn from_namespace(namespace: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|v| v.namespace() == namespace)
    }
}

/// Detect the AUTOSAR release of an ARXML document.
pub fn detect_version(data: &[u8]) -> Option<ArxmlVersion> {
    detect_namespace(data).and_then(|ns| ArxmlVersion::from_namespace(&ns))
}

/// A parsed ARXML document, independent of the concrete schema release.
#[derive(Debug)]
pub enum Model {
    /// AUTOSAR R4.0 document.
    #[cfg(feature = "r4-0")]
    R4_0(arxml_model_r4_0::AutosarType),
}

impl Model {
    /// Read a document and dispatch it to the matching model.
    pub fn read(data: &[u8]) -> Result<Self, ArxmlError> {
        let version =
            detect_version(data).ok_or_else(|| ArxmlError::UnsupportedVersion(format!(
                "unknown AUTOSAR namespace: {:?}",
                detect_namespace(data)
            )))?;

        match version {
            #[cfg(feature = "r4-0")]
            ArxmlVersion::R4_0 => Ok(Self::R4_0(arxml_model_r4_0::api::read_from_slice(
                data,
            )?)),
            #[allow(unreachable_patterns)]
            _ => Err(ArxmlError::UnsupportedVersion(format!(
                "AUTOSAR {} is not enabled in this build",
                version.label()
            ))),
        }
    }

    /// Write the document to a writer.
    pub fn write<W: Write>(&self, writer: W) -> Result<(), ArxmlError> {
        match self {
            #[cfg(feature = "r4-0")]
            Self::R4_0(model) => arxml_model_r4_0::api::write(model, writer),
            #[allow(unreachable_patterns)]
            _ => Err(ArxmlError::UnsupportedVersion(
                "no AUTOSAR release enabled in this build".to_string(),
            )),
        }
    }

    /// Serialize the document into a `String`.
    pub fn to_string(&self) -> Result<String, ArxmlError> {
        match self {
            #[cfg(feature = "r4-0")]
            Self::R4_0(model) => arxml_model_r4_0::api::to_string(model),
            #[allow(unreachable_patterns)]
            _ => Err(ArxmlError::UnsupportedVersion(
                "no AUTOSAR release enabled in this build".to_string(),
            )),
        }
    }

    /// Validate the document against its schema.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            #[cfg(feature = "r4-0")]
            Self::R4_0(model) => arxml_model_r4_0::api::validate(model),
            #[allow(unreachable_patterns)]
            _ => Err("no AUTOSAR release enabled in this build".to_string()),
        }
    }
}

/// Release specific helpers (one module per supported schema version).
pub mod versions {
    #[cfg(feature = "r4-0")]
    pub mod r4_0 {
        pub use arxml_model_r4_0::{api, model, Autosar, AutosarType, AUTOSAR_NS, ROOT_ELEMENT};

        /// Read an AUTOSAR R4.0 document from a stream.
        pub fn read<R: std::io::BufRead>(
            reader: R,
        ) -> Result<AutosarType, arxml_core::ArxmlError> {
            api::read(reader)
        }

        /// Write an AUTOSAR R4.0 document to a writer.
        pub fn write<W: std::io::Write>(
            model: &AutosarType,
            writer: W,
        ) -> Result<(), arxml_core::ArxmlError> {
            api::write(model, writer)
        }

        /// Validate an AUTOSAR R4.0 document.
        pub fn validate(model: &AutosarType) -> Result<(), String> {
            api::validate(model)
        }
    }
}
