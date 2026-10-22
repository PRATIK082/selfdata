//! `arxml-cli` – a small, **schema independent** ARXML tool.
//!
//! It works on the generic DOM from [`arxml_core::dom`], therefore it does not
//! need any generated code and works with every AUTOSAR release.
//!
//! ```text
//! arxml-cli version  <file>                     # root element + namespace
//! arxml-cli get      <file> <path>              # read values by path
//! arxml-cli set      <file> <path> <value> [-o] # update values by path
//! arxml-cli validate <file>                     # generic structural checks
//! ```
//!
//! Path syntax: `AR-PACKAGES/AR-PACKAGE/SHORT-NAME`, `AR-PACKAGES/AR-PACKAGE[2]/SHORT-NAME`,
//! `AR-PACKAGES/*/SHORT-NAME`.

use std::fs::File;
use std::io::{BufReader, Write};
use std::process::ExitCode;

use arxml_core::dom::Document;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let result = match args.first().map(String::as_str) {
        Some("version") => cmd_version(&args),
        Some("get") => cmd_get(&args),
        Some("set") => cmd_set(&args),
        Some("validate") => cmd_validate(&args),
        Some("help") | None => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("unknown command '{other}' (try 'help')")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "usage:
  arxml-cli version  <file>
  arxml-cli get      <file> <path>
  arxml-cli set      <file> <path> <value> [-o <out>]
  arxml-cli validate <file>"
    );
}

fn load(path: &str) -> Result<Document, String> {
    let file = File::open(path).map_err(|err| format!("cannot open '{path}': {err}"))?;
    Document::parse(BufReader::new(file)).map_err(|err| format!("cannot parse '{path}': {err}"))
}

fn cmd_version(args: &[String]) -> Result<(), String> {
    let path = args.get(1).ok_or("missing <file>")?;
    let doc = load(path)?;
    println!("root      : {}", doc.root.name);
    println!("namespace : {}", doc.root.namespace().unwrap_or("-"));
    Ok(())
}

fn cmd_get(args: &[String]) -> Result<(), String> {
    let path = args.get(1).ok_or("missing <file>")?;
    let query = args.get(2).ok_or("missing <path>")?;
    let doc = load(path)?;

    let texts = doc.texts_at(query);
    if texts.is_empty() {
        println!("(no match for '{query}')");
    } else {
        for text in texts {
            println!("{text}");
        }
    }
    Ok(())
}

fn cmd_set(args: &[String]) -> Result<(), String> {
    let path = args.get(1).ok_or("missing <file>")?;
    let query = args.get(2).ok_or("missing <path>")?;
    let value = args.get(3).ok_or("missing <value>")?;
    let out = args
        .iter()
        .position(|arg| arg == "-o")
        .and_then(|idx| args.get(idx + 1))
        .cloned()
        .unwrap_or_else(|| path.clone());

    let mut doc = load(path)?;
    let updated = doc.set_text_at(query, value);
    if updated == 0 {
        return Err(format!("path '{query}' did not match any node"));
    }

    let xml = doc
        .to_xml_string()
        .map_err(|err| format!("cannot serialize: {err}"))?;
    let mut file = File::create(&out).map_err(|err| format!("cannot write '{out}': {err}"))?;
    file.write_all(xml.as_bytes())
        .map_err(|err| format!("cannot write '{out}': {err}"))?;
    println!("updated {updated} node(s) -> {out}");
    Ok(())
}

fn cmd_validate(args: &[String]) -> Result<(), String> {
    let path = args.get(1).ok_or("missing <file>")?;
    let doc = load(path)?;
    let issues = doc.validate();

    if issues.is_empty() {
        println!("ok");
        return Ok(());
    }

    for issue in &issues {
        println!("{:?} at {} – {}", issue.kind, issue.path, issue.message);
    }
    Err(format!("{} issue(s) found", issues.len()))
}
