//! `xsd-api-gen` -- draft generator that turns an AUTOSAR XSD into a Rust
//! API module with **one entry per XML element**.
//!
//! This is the starting point for the new development line
//! "XSD -> Rust API generation for each Autosar element". The output is
//! intentionally a *draft*: it only emits empty structs + trait impls and a
//! list of element names. The real typed getters/setters will be filled in
//! later, driven by the generated model (`arxml-index`) and the DOM layer.
//!
//! ```text
//! xsd-api-gen <schema.xsd> <output.rs>
//! ```

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let mut args = env::args().skip(1);
    let schema = args.next().unwrap_or_else(|| {
        eprintln!("usage: xsd-api-gen <schema.xsd> <output.rs>");
        std::process::exit(2);
    });
    let output = args.next().unwrap_or_else(|| {
        eprintln!("usage: xsd-api-gen <schema.xsd> <output.rs>");
        std::process::exit(2);
    });

    let input = match fs::read_to_string(&schema) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("cannot read '{}': {}", schema, err);
            std::process::exit(1);
        }
    };

    let elements = collect_elements(&input);
    let code = render(&elements);

    if let Some(parent) = PathBuf::from(&output).parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(err) = fs::write(&output, code) {
        eprintln!("cannot write '{}': {}", output, err);
        std::process::exit(1);
    }
    eprintln!("wrote {} ({} elements)", output, elements.len());
}

/// Collect the names of all `xs(d):element` declarations in document order,
/// de-duplicated.
fn collect_elements(input: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut ordered = Vec::new();

    for line in input.lines() {
        // Only lines that declare an element.
        if !line.contains("element") {
            continue;
        }
        // Extract name="..." (single or double quotes).
        if let Some(name) = extract_attr(line, "name") {
            if name.is_empty() {
                continue;
            }
            // Heuristic: skip anonymous / unnamed / internal helper elements.
            if name.starts_with('_') {
                continue;
            }
            if seen.insert(name.clone()) {
                ordered.push(name);
            }
        }
    }
    ordered
}

/// Extract the value of `attr="value"` or `attr='value'` from a fragment.
fn extract_attr(fragment: &str, attr: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let needle = format!("{attr}={quote}");
        if let Some(start) = fragment.find(&needle) {
            let rest = &fragment[start + needle.len()..];
            if let Some(end) = rest.find(quote) {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

/// Turn an XML element name (e.g. `AR-PACKAGE`) into a valid Rust type
/// name (e.g. `ArPackage`).
fn rust_type_name(xml_name: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;
    for ch in xml_name.chars() {
        if ch.is_alphanumeric() {
            if capitalize {
                for c in ch.to_uppercase() {
                    result.push(c);
                }
                capitalize = false;
            } else {
                for c in ch.to_lowercase() {
                    result.push(c);
                }
            }
        } else {
            capitalize = true;
        }
    }
    if result.is_empty() {
        result.push_str("Element");
    }
    // Make sure it does not start with a digit.
    if result.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        result.insert(0, 'E');
    }
    result
}

fn render(elements: &[String]) -> String {
    let mut out = String::new();
    out.push_str("// Draft Rust API generated from an AUTOSAR XSD.\n");
    out.push_str("// Status: DRAFT -- empty stubs, to be implemented.\n");
    out.push_str("// Generator: tools/xsd-api-gen\n\n");

    out.push_str("/// Common trait implemented by every generated element API.\n");
    out.push_str("pub trait AutosarElementApi {\n");
    out.push_str("    /// XML element name this API represents.\n");
    out.push_str("    fn element_name() -> &'static str;\n");
    out.push_str("    /// Short human readable description (draft).\n");
    out.push_str("    fn description() -> &'static str { \"\" }\n");
    out.push_str("}\n\n");

    out.push_str("pub mod elements {\n");
    for name in elements {
        let ty = rust_type_name(name);
        out.push_str(&format!("    /// Draft API for `<{name}>`.\n"));
        out.push_str(&format!("    pub struct {ty};\n"));
        out.push_str(&format!("    impl super::AutosarElementApi for {ty} {{\n"));
        out.push_str(&format!("        fn element_name() -> &'static str {{ {:?} }}\n", name));
        out.push_str("    }\n\n");
    }
    out.push_str("}\n\n");

    out.push_str("/// All element names discovered in the schema, in document order.\n");
    out.push_str("pub const ELEMENT_NAMES: &[&str] = &[\n");
    for name in elements {
        out.push_str(&format!("    {:?},\n", name));
    }
    out.push_str("];\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_name() {
        assert_eq!(extract_attr(r#"xs:element name="AR-PACKAGE" type="t""#, "name"), Some("AR-PACKAGE".to_string()));
        assert_eq!(extract_attr(r#"xs:element name='FOO'/"#, "name"), Some("FOO".to_string()));
    }

    #[test]
    fn type_name_conversion() {
        assert_eq!(rust_type_name("AR-PACKAGE"), "ArPackage");
        assert_eq!(rust_type_name("SHORT-NAME"), "ShortName");
        assert_eq!(rust_type_name("a_b"), "AB"); // not ideal but acceptable draft
    }

    #[test]
    fn collects_elements() {
        let xsd = r#"
            <xs:element name="AUTOSAR" type="AUTOSAR"/>
            <xs:element name="AR-PACKAGES"/>
        "#;
        let els = collect_elements(xsd);
        assert_eq!(els, vec!["AUTOSAR".to_string(), "AR-PACKAGES".to_string()]);
    }
}