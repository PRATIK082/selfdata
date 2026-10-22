//! Read → mutate → write → validate an ARXML document with the generated
//! AUTOSAR R4.0 model.
//!
//! Run with:
//!
//! ```text
//! cargo run -p arxml --example read_write_validate
//! ```

use std::fs::File;
use std::io::{BufReader, BufWriter};

use arxml::versions::r4_0::{model, AutosarType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ---------------------------------------------------------------- read
    let sample = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("data")
        .join("sample.arxml");

    println!("reading {}", sample.display());
    let file = File::open(&sample)?;
    let model: AutosarType = arxml::versions::r4_0::read(BufReader::new(file))?;
    println!("  parsed AUTOSAR document");

    // ------------------------------------------------------------ validate
    arxml::versions::r4_0::validate(&model)?;
    println!("  schema validation: OK");

    // -------------------------------------------------------------- mutate
    if let Some(packages) = model.content.ar_packages.as_ref() {
        for entry in &packages.content {
            if let model::ArPackages6ElementTypeContent::ArPackage(package) = entry {
                package.content.short_name.content.0 = "RenamedPackage".to_string();
                println!(
                    "  renamed package to {:?}",
                    package.content.short_name.content.0
                );
            }
        }
    }

    // --------------------------------------------------------------- write
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("sample.out.arxml");
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(&target)?;
    arxml::versions::r4_0::write(&model, BufWriter::new(file))?;
    println!("  wrote {}", target.display());

    // ------------------------------------------------------ re-read/validate
    let bytes = std::fs::read(&target)?;
    println!(
        "  detected AUTOSAR version: {:?}",
        arxml::detect_version(&bytes).map(|v| v.label())
    );
    let reread = arxml::versions::r4_0::read(BufReader::new(&bytes[..]))?;
    arxml::versions::r4_0::validate(&reread)?;
    println!("  round trip validation: OK");

    // The version independent facade works the same way.
    let version_agnostic = arxml::Model::read(&bytes)?;
    version_agnostic.validate()?;
    println!("  version independent validation: OK");

    Ok(())
}
