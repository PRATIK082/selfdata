//! A small, generic ARXML document object model.
//!
//! This module is **completely independent of the generated model code**: it
//! only uses `quick-xml` and therefore works for *any* AUTOSAR release (and
//! even for documents that are not schema valid).
//!
//! It exists because the generated model contains thousands of structs with
//! very similar field groups (`SHORT-NAME`, `LONG-NAME`, `DESC`, …). Reading
//! 200k lines of generated code to write per-type update code is not
//! practical, so generic tooling works on paths instead:
//!
//! ```
//! use arxml_core::dom::Document;
//!
//! let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
//! <AUTOSAR xmlns="http://autosar.org/schema/r4.0">
//!   <AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Old</SHORT-NAME></AR-PACKAGE></AR-PACKAGES>
//! </AUTOSAR>"#;
//!
//! let mut doc = Document::parse_str(xml).unwrap();
//! assert_eq!(doc.text_at("/AR-PACKAGES/AR-PACKAGE/SHORT-NAME"), Some("Old"));
//! assert_eq!(doc.set_text_at("/AR-PACKAGES/AR-PACKAGE/SHORT-NAME", "New"), 1);
//! assert_eq!(doc.text_at("/AR-PACKAGES/AR-PACKAGE/SHORT-NAME"), Some("New"));
//! ```
//!
//! Paths use a simple syntax:
//!
//! * `/AR-PACKAGES/AR-PACKAGE/SHORT-NAME` – child chain, the first segment is
//!   the root element itself.
//! * `*` – any child element.
//! * `AR-PACKAGE[2]` – the second child element with that name (1 based).

use std::collections::HashMap;
use std::io::{BufRead, Write};

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{escape, Reader, Writer};

use crate::ArxmlError;

/// A single element of an ARXML document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Node {
    /// Local name of the element (without namespace prefix).
    pub name: String,
    /// Attributes of the element, in document order.
    pub attributes: Vec<Attribute>,
    /// Child elements.
    pub children: Vec<Node>,
    /// Text content of the element (trimmed).
    pub text: String,
}

/// An XML attribute of a [`Node`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// Attribute name as written in the document (may include a prefix).
    pub name: String,
    /// Unescaped attribute value.
    pub value: String,
}

impl Node {
    /// Returns the value of the attribute with the given name (prefix aware).
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attrib| attrib.name == name)
            .map(|attrib| attrib.value.as_str())
    }

    /// Returns the value of the `xmlns` attribute (default namespace).
    pub fn namespace(&self) -> Option<&str> {
        self.attribute("xmlns")
    }

    /// Returns all direct children with the given local name.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |node| node.name == name)
    }

    /// Returns the first direct child with the given local name.
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|node| node.name == name)
    }
}

/// A parsed ARXML document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The root element of the document.
    pub root: Node,
}

/// A single problem found by [`Document::validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// Machine readable kind of the problem.
    pub kind: IssueKind,
    /// Path of the offending element (empty for document level problems).
    pub path: String,
    /// Human readable description.
    pub message: String,
}

/// The kinds of problems reported by [`Document::validate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    /// The root element is not `AUTOSAR`.
    UnexpectedRoot,
    /// The document has no default namespace.
    MissingNamespace,
    /// The default namespace is not an AUTOSAR namespace.
    UnexpectedNamespace,
    /// A `SHORT-NAME` element is empty.
    EmptyShortName,
    /// The same `UUID` attribute value is used more than once.
    DuplicateUuid,
    /// An element contains both text and child elements.
    MixedContent,
}

impl Issue {
    /// Creates a new issue.
    pub fn new(kind: IssueKind, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind,
            path: path.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

impl Document {
    /// Parses an ARXML document from a reader.
    pub fn parse<R: BufRead>(reader: R) -> Result<Self, ArxmlError> {
        let mut xml = Reader::from_reader(reader);
        xml.config_mut().trim_text(true);

        let mut stack: Vec<Node> = Vec::new();
        let mut root: Option<Node> = None;
        let mut buffer = Vec::new();

        loop {
            let event = xml
                .read_event_into(&mut buffer)
                .map_err(|err| ArxmlError::Xml(err.into()))?;
            match event {
                Event::Start(start) => {
                    stack.push(node_from_start(&start)?);
                }
                Event::Empty(start) => {
                    let node = node_from_start(&start)?;
                    push_node(&mut stack, &mut root, node);
                }
                Event::Text(text) => {
                    if let Some(node) = stack.last_mut() {
                        let raw = text.decode().map_err(xml_error)?;
        let value = escape::unescape(&raw).map_err(xml_error)?.into_owned();
                        node.text.push_str(&value);
                    }
                }
                Event::CData(text) => {
                    if let Some(node) = stack.last_mut() {
                        let value = String::from_utf8_lossy(text.as_ref()).into_owned();
                        node.text.push_str(&value);
                    }
                }
                Event::End(_) => {
                    if let Some(node) = stack.pop() {
                        push_node(&mut stack, &mut root, node);
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buffer.clear();
        }

        match root {
            Some(root) => Ok(Self { root }),
            None => Err(ArxmlError::Validation(
                "document does not contain a root element".to_string(),
            )),
        }
    }

    /// Parses an ARXML document from a string.
    pub fn parse_str(xml: &str) -> Result<Self, ArxmlError> {
        Self::parse(xml.as_bytes())
    }

    /// Parses an ARXML document from a byte slice.
    pub fn parse_bytes(xml: &[u8]) -> Result<Self, ArxmlError> {
        Self::parse(xml)
    }

    /// Selects all elements matching the given path.
    ///
    /// The first segment may either be the name of the root element itself or
    /// the name of a child of the root element:
    ///
    /// * `/AR-PACKAGES/AR-PACKAGE/SHORT-NAME`
    /// * `/AUTOSAR/AR-PACKAGES/AR-PACKAGE/SHORT-NAME`
    pub fn select(&self, path: &str) -> Vec<&Node> {
        let segments = parse_path(path);
        let mut current: Vec<&Node> = vec![&self.root];
        let mut first = true;
        for segment in &segments {
            if first {
                first = false;
                if segment.matches_name(&self.root) {
                    continue;
                }
            }
            current = matching_children(current, segment);
        }
        current
    }

    /// Returns the text of the first element matching the given path.
    pub fn text_at(&self, path: &str) -> Option<&str> {
        self.select(path).first().map(|node| node.text.as_str())
    }

    /// Returns the text of all elements matching the given path.
    pub fn texts_at(&self, path: &str) -> Vec<&str> {
        self.select(path)
            .into_iter()
            .map(|node| node.text.as_str())
            .collect()
    }

    /// Replaces the text of all elements matching the given path.
    ///
    /// Returns the number of updated elements.
    pub fn set_text_at(&mut self, path: &str, value: &str) -> usize {
        let segments = parse_path(path);
        let root_matches = segments
            .first()
            .map(|segment| segment.matches_name(&self.root))
            .unwrap_or(false);
        let mut current: Vec<&mut Node> = vec![&mut self.root];
        let mut first = true;
        for segment in &segments {
            if first {
                first = false;
                if root_matches {
                    continue;
                }
            }
            current = matching_children_mut(current, segment);
        }
        let count = current.len();
        for node in current {
            node.text = value.to_string();
        }
        count
    }

    /// Writes the document back to XML.
    pub fn write_to<W: Write>(&self, writer: W) -> Result<(), ArxmlError> {
        let mut writer = Writer::new(writer);
        write_node(&mut writer, &self.root)?;
        writer.into_inner().flush().map_err(ArxmlError::Io)
    }

    /// Serializes the document into a `String`.
    pub fn to_xml_string(&self) -> Result<String, ArxmlError> {
        let mut buffer = Vec::new();
        self.write_to(&mut buffer)?;
        String::from_utf8(buffer).map_err(|err| {
            ArxmlError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                err.to_string(),
            ))
        })
    }

    /// Runs the generic (schema independent) AUTOSAR checks.
    ///
    /// These checks do not need the generated model and therefore work for
    /// every AUTOSAR release:
    ///
    /// * the root element is `AUTOSAR`
    /// * the document declares a default AUTOSAR namespace
    /// * every `SHORT-NAME` is not empty
    /// * `UUID` attributes are unique
    /// * no element mixes text and child elements
    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = Vec::new();

        if self.root.name != crate::AUTOSAR_ROOT {
            issues.push(Issue::new(
                IssueKind::UnexpectedRoot,
                String::new(),
                format!(
                    "expected root element <{}>, found <{}>",
                    crate::AUTOSAR_ROOT, self.root.name
                ),
            ));
        }

        match self.root.namespace() {
            None => issues.push(Issue::new(
                IssueKind::MissingNamespace,
                String::new(),
                "document does not declare a default namespace".to_string(),
            )),
            Some(ns) if !ns.starts_with("http://autosar.org/schema/") => {
                issues.push(Issue::new(
                    IssueKind::UnexpectedNamespace,
                    String::new(),
                    format!("expected an AUTOSAR namespace, found '{ns}'"),
                ));
            }
            Some(_) => {}
        }

        let mut uuids: HashMap<&str, String> = HashMap::new();
        self.collect_issues(&self.root, "", &mut uuids, &mut issues);
        issues
    }

    fn collect_issues<'a>(
        &'a self,
        node: &'a Node,
        path: &str,
        uuids: &mut HashMap<&'a str, String>,
        issues: &mut Vec<Issue>,
    ) {
        let path = if path.is_empty() {
            format!("/{}", node.name)
        } else {
            format!("{path}/{}", node.name)
        };

        if node.name == "SHORT-NAME" && node.text.trim().is_empty() {
            issues.push(Issue::new(
                IssueKind::EmptyShortName,
                path.clone(),
                "SHORT-NAME must not be empty".to_string(),
            ));
        }

        if !node.text.trim().is_empty() && !node.children.is_empty() {
            issues.push(Issue::new(
                IssueKind::MixedContent,
                path.clone(),
                "element contains both text and child elements".to_string(),
            ));
        }

        if let Some(uuid) = node.attribute("UUID") {
            if let Some(previous) = uuids.insert(uuid, path.clone()) {
                issues.push(Issue::new(
                    IssueKind::DuplicateUuid,
                    path.clone(),
                    format!("UUID '{uuid}' is already used by '{previous}'"),
                ));
            }
        }

        for child in &node.children {
            self.collect_issues(child, &path, uuids, issues);
        }
    }
}

fn node_from_start(start: &BytesStart) -> Result<Node, ArxmlError> {
    let name = String::from_utf8_lossy(start.local_name().as_ref()).into_owned();
    let mut attributes = Vec::new();
    for attrib in start.attributes() {
        let attrib = attrib.map_err(xml_error)?;
        let key = String::from_utf8_lossy(attrib.key.as_ref()).into_owned();
        let raw = String::from_utf8_lossy(attrib.value.as_ref()).into_owned();
        let value = escape::unescape(&raw).map_err(xml_error)?.into_owned();
        attributes.push(Attribute { name: key, value });
    }
    Ok(Node {
        name,
        attributes,
        children: Vec::new(),
        text: String::new(),
    })
}

/// Converts a `quick-xml` error into an [`ArxmlError`].
fn xml_error<E>(error: E) -> ArxmlError
where
    E: Into<xsd_parser_types::quick_xml::Error>,
{
    ArxmlError::Xml(error.into())
}

fn push_node(stack: &mut [Node], root: &mut Option<Node>, node: Node) {
    match stack.last_mut() {
        Some(parent) => parent.children.push(node),
        None => {
            if root.is_none() {
                *root = Some(node);
            }
        }
    }
}

fn write_node<W: Write>(writer: &mut Writer<W>, node: &Node) -> Result<(), ArxmlError> {
    let mut start = BytesStart::new(node.name.as_str());
    for attrib in &node.attributes {
        let escaped = quick_xml::escape::escape(&attrib.value);
        start.push_attribute((attrib.name.as_str(), escaped));
    }

    if node.children.is_empty() && node.text.is_empty() {
        writer
            .write_event(Event::Empty(start))
            .map_err(ArxmlError::Io)?;
        return Ok(());
    }

    writer
        .write_event(Event::Start(start))
        .map_err(ArxmlError::Io)?;
    if !node.text.is_empty() {
        writer
            .write_event(Event::Text(BytesText::new(node.text.as_str())))
            .map_err(ArxmlError::Io)?;
    }
    for child in &node.children {
        write_node(writer, child)?;
    }
    writer
        .write_event(Event::End(BytesEnd::new(node.name.as_str())))
        .map_err(ArxmlError::Io)?;
    Ok(())
}

/// A single segment of a path expression.
#[derive(Debug, Clone)]
struct Segment {
    name: String,
    wildcard: bool,
    index: Option<usize>,
}

impl Segment {
    fn matches_name(&self, node: &Node) -> bool {
        self.wildcard || node.name == self.name
    }
}

fn matching_children<'a>(nodes: Vec<&'a Node>, segment: &Segment) -> Vec<&'a Node> {
    let mut result = Vec::new();
    for node in nodes {
        let mut counter = 0usize;
        for child in &node.children {
            if !segment.matches_name(child) {
                continue;
            }
            counter += 1;
            if let Some(index) = segment.index {
                if counter != index {
                    continue;
                }
            }
            result.push(child);
        }
    }
    result
}

fn matching_children_mut<'a>(
    nodes: Vec<&'a mut Node>,
    segment: &Segment,
) -> Vec<&'a mut Node> {
    let mut result = Vec::new();
    for node in nodes {
        let mut counter = 0usize;
        for child in &mut node.children {
            if !segment.matches_name(child) {
                continue;
            }
            counter += 1;
            if let Some(index) = segment.index {
                if counter != index {
                    continue;
                }
            }
            result.push(child);
        }
    }
    result
}

fn parse_path(path: &str) -> Vec<Segment> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            if segment == "*" {
                return Segment {
                    name: String::new(),
                    wildcard: true,
                    index: None,
                };
            }
            if let Some(open) = segment.find('[') {
                let name = segment[..open].to_string();
                let index = segment[open + 1..]
                    .trim_end_matches(']')
                    .trim()
                    .parse::<usize>()
                    .ok();
                Segment {
                    name,
                    wildcard: false,
                    index,
                }
            } else {
                Segment {
                    name: segment.to_string(),
                    wildcard: false,
                    index: None,
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<AUTOSAR xmlns="http://autosar.org/schema/r4.0">
  <AR-PACKAGES>
    <AR-PACKAGE UUID="11111111-1111-1111-1111-111111111111">
      <SHORT-NAME>First</SHORT-NAME>
    </AR-PACKAGE>
    <AR-PACKAGE UUID="22222222-2222-2222-2222-222222222222">
      <SHORT-NAME>Second</SHORT-NAME>
    </AR-PACKAGE>
  </AR-PACKAGES>
</AUTOSAR>"#;

    #[test]
    fn parses_and_selects() {
        let doc = Document::parse_str(SAMPLE).unwrap();
        assert_eq!(doc.root.name, "AUTOSAR");
        assert_eq!(
            doc.root.namespace(),
            Some("http://autosar.org/schema/r4.0")
        );
        assert_eq!(
            doc.texts_at("/AR-PACKAGES/AR-PACKAGE/SHORT-NAME"),
            vec!["First", "Second"]
        );
        assert_eq!(
            doc.text_at("/AR-PACKAGES/AR-PACKAGE[2]/SHORT-NAME"),
            Some("Second")
        );
    }

    #[test]
    fn updates_text() {
        let mut doc = Document::parse_str(SAMPLE).unwrap();
        assert_eq!(
            doc.set_text_at("/AR-PACKAGES/AR-PACKAGE[1]/SHORT-NAME", "Renamed"),
            1
        );
        assert_eq!(
            doc.text_at("/AR-PACKAGES/AR-PACKAGE[1]/SHORT-NAME"),
            Some("Renamed")
        );
    }

    #[test]
    fn round_trips() {
        let doc = Document::parse_str(SAMPLE).unwrap();
        let xml = doc.to_xml_string().unwrap();
        let again = Document::parse_str(&xml).unwrap();
        assert_eq!(doc, again);
    }

    #[test]
    fn validates() {
        let doc = Document::parse_str(SAMPLE).unwrap();
        assert!(doc.validate().is_empty());

        let broken = SAMPLE.replace("First", "");
        let doc = Document::parse_str(&broken).unwrap();
        let issues = doc.validate();
        assert!(issues
            .iter()
            .any(|issue| issue.kind == IssueKind::EmptyShortName));
    }
}
