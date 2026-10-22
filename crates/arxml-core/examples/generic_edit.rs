//! Generic ARXML editing – **without** touching the generated model code.
//!
//! The DOM in [`arxml_core::dom`] is built directly on `quick-xml`, so it works
//! for every AUTOSAR release and does not need the (very large) generated model
//! crate. Fields are addressed by path instead of by Rust struct field:
//!
//! ```text
//! AR-PACKAGES/AR-PACKAGE/SHORT-NAME      → first package
//! AR-PACKAGES/AR-PACKAGE[2]/SHORT-NAME   → second package
//! AR-PACKAGES/*/SHORT-NAME               → all packages
//! ```
//!
//! Run with:
//!
//! ```text
//! cargo run -p arxml-core --example generic_edit -- crates/arxml/examples/data/sample.arxml
//! ```

use std::fs::File;
use std::io::BufReader;

use arxml_core::dom::Document;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "crates/arxml/examples/data/sample.arxml".to_string());

    // ---------------------------------------------------------------- read
    let file = File::open(&path)?;
    let mut doc = Document::parse(BufReader::new(file))?;
    println!("root element : {}", doc.root.name);
    println!("namespace    : {:?}", doc.root.namespace().unwrap_or("-"));

    // --------------------------------------------------------------- query
    let names = doc.texts_at("AR-PACKAGES/AR-PACKAGE/SHORT-NAME");
    println!("short names  : {names:?}");

    if let Some(first) = doc.text_at("AR-PACKAGES/AR-PACKAGE/SHORT-NAME") {
        println!("first package: {first}");
    }

    // -------------------------------------------------------------- update
    let updated = doc.set_text_at("AR-PACKAGES/AR-PACKAGE/SHORT-NAME", "RenamedByGenericLayer");
    println!("updated      : {updated} node(s)");

    // --------------------------------------------------------------- write
    let out = std::path::Path::new("target/generic-edit.arxml");
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, doc.to_xml_string()?)?;
    println!("written      : {}", out.display());

    // ------------------------------------------------------------ validate
    let issues = doc.validate();
    if issues.is_empty() {
        println!("validation   : ok");
    } else {
        for issue in &issues {
            println!("validation   : {:?} at {} – {}", issue.kind, issue.path, issue.message);
        }
    }

    Ok(())
}
