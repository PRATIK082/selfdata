//! `arxml-index` builds a searchable index of the *generated* AUTOSAR model
//! code.
//!
//! The generated model contains thousands of structs that share very similar
//! field groups (`SHORT-NAME`, `LONG-NAME`, `DESC`, `ADMIN-DATA`, …). Reading
//! the 130+ MB of generated Rust to find out "which types have a
//! `short_name_fragments` field" is not practical, so this tool scans the
//! generated file once and writes a compact JSON index:
//!
//! ```text
//! arxml-index <generated.rs> [-o model-index.json]
//! ```
//!
//! The index can then be queried with any JSON tool, e.g. to find all types
//! that contain a given field.
//!
//! The tool is intentionally dependency free (apart from `serde_json`) and does
//! not need the XSD – it only understands the shape of the code that
//! `xsd-parser` emits.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use serde_json::{json, Map, Value};

/// A single field of a generated struct.
#[derive(Debug, Clone)]
struct Field {
    name: String,
    type_name: String,
}

/// The result of scanning a generated model file.
struct Index {
    /// `module::TypeName` → fields.
    types: BTreeMap<String, Vec<Field>>,
    /// field name → types that contain it.
    field_usage: BTreeMap<String, Vec<String>>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => {
                output = Some(PathBuf::from(
                    args.next().ok_or("missing value for --output")?,
                ));
            }
            "-h" | "--help" => {
                println!("usage: arxml-index <generated.rs> [-o <index.json>]");
                return Ok(());
            }
            other => {
                if input.is_some() {
                    return Err(format!("unexpected argument '{other}'"));
                }
                input = Some(PathBuf::from(other));
            }
        }
    }

    let input = input.ok_or("usage: arxml-index <generated.rs> [-o <index.json>]")?;
    let file = File::open(&input).map_err(|err| format!("{}: {err}", input.display()))?;
    let reader = BufReader::new(file);

    let index = build_index(reader)?;
    let json = serde_json::to_string_pretty(&index.to_json()).map_err(|err| err.to_string())?;

    match &output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .map_err(|err| format!("{}: {err}", parent.display()))?;
                }
            }
            let mut file = File::create(path).map_err(|err| format!("{}: {err}", path.display()))?;
            file.write_all(json.as_bytes())
                .map_err(|err| format!("{}: {err}", path.display()))?;
            file.write_all(b"\n")
                .map_err(|err| format!("{}: {err}", path.display()))?;
            println!(
                "wrote {} ({} types, {} distinct fields)",
                path.display(),
                index.types.len(),
                index.field_usage.len()
            );
        }
        None => println!("{json}"),
    }

    Ok(())
}

impl Index {
    fn to_json(&self) -> Value {
        let mut types_map = Map::new();
        for (name, fields) in &self.types {
            let mut field_map = Map::new();
            for field in fields {
                field_map.insert(field.name.clone(), Value::String(field.type_name.clone()));
            }
            types_map.insert(name.clone(), Value::Object(field_map));
        }

        let mut usage_map = Map::new();
        for (field, owners) in &self.field_usage {
            usage_map.insert(
                field.clone(),
                Value::Array(owners.iter().map(|o| Value::String(o.clone())).collect()),
            );
        }

        json!({
            "types": Value::Object(types_map),
            "field_usage": Value::Object(usage_map),
        })
    }
}

/// Scans the generated Rust file and extracts every `pub struct` with its
/// public fields.
fn build_index<R: BufRead>(reader: R) -> Result<Index, String> {
    let mut types: BTreeMap<String, Vec<Field>> = BTreeMap::new();
    let mut field_usage: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // Open modules with the brace depth at which their body starts.
    let mut modules: Vec<(String, i64)> = Vec::new();
    // The struct that is currently being read.
    let mut in_struct = false;
    let mut struct_name = String::new();
    let mut struct_module = String::new();
    let mut struct_fields: Vec<Field> = Vec::new();
    let mut struct_depth: i64 = 0;
    let mut depth: i64 = 0;

    for line in reader.lines() {
        let line = line.map_err(|err| err.to_string())?;
        let trimmed = line.trim();
        let opens = trimmed.matches('{').count() as i64;
        let closes = trimmed.matches('}').count() as i64;

        if in_struct {
            for field in parse_fields(trimmed) {
                field_usage
                    .entry(field.name.clone())
                    .or_default()
                    .push(struct_name.clone());
                struct_fields.push(field);
            }
            depth += opens - closes;
            if depth < struct_depth {
                in_struct = false;
                types.insert(
                    format!("{struct_module}::{struct_name}"),
                    std::mem::take(&mut struct_fields),
                );
            }
            continue;
        }

        if trimmed.starts_with("pub mod ") || trimmed.starts_with("mod ") {
            let name = trimmed
                .trim_start_matches("pub mod ")
                .trim_start_matches("mod ")
                .trim()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(|c: char| c == '{' || c == '}' || c == ';')
                .to_string();
            modules.push((name, depth + 1));
            depth += opens - closes;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("pub struct ") {
            let name = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(|c: char| c == '{' || c == '}' || c == ';')
                .to_string();
            let module = modules
                .last()
                .map(|(name, _)| name.clone())
                .unwrap_or_default();

            let mut fields = parse_fields(rest);
            for field in &fields {
                field_usage
                    .entry(field.name.clone())
                    .or_default()
                    .push(name.clone());
            }

            if opens > closes {
                depth += opens - closes;
                in_struct = true;
                struct_name = name;
                struct_module = module;
                struct_fields = fields;
                struct_depth = depth;
            } else {
                depth += opens - closes;
                types.insert(format!("{module}::{name}"), fields);
            }
            continue;
        }

        depth += opens - closes;
        while let Some((_, inner)) = modules.last() {
            if depth < *inner {
                modules.pop();
            } else {
                break;
            }
        }
    }

    Ok(Index {
        types,
        field_usage,
    })
}

/// Extracts `pub <name>: <Type>` pairs from a fragment of Rust code.
fn parse_fields(fragment: &str) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut rest = fragment;

    while let Some(pos) = rest.find("pub ") {
        let tail = &rest[pos + 4..];
        let tail = tail.trim_start();
        let name_end = tail
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(tail.len());
        if name_end == 0 {
            rest = tail;
            continue;
        }
        let name = tail[..name_end].to_string();
        let after_name = tail[name_end..].trim_start();
        let Some(after_colon) = after_name.strip_prefix(':') else {
            rest = after_name;
            continue;
        };
        let type_src = after_colon.trim_start();

        // The type ends at the next top level `,` or `;`.
        let mut end = type_src.len();
        let mut nesting = 0i32;
        for (idx, ch) in type_src.char_indices() {
            match ch {
                '<' | '(' | '[' => nesting += 1,
                '>' | ')' | ']' => nesting -= 1,
                ',' | ';' if nesting == 0 => {
                    end = idx;
                    break;
                }
                _ => {}
            }
        }
        let type_name = type_src[..end].trim().to_string();
        if !name.is_empty() && !type_name.is_empty() {
            fields.push(Field { name, type_name });
        }
        rest = &type_src[end..];
    }

    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
pub mod ar {
    pub struct ArPackageTypeContent {
        pub short_name: IdentifierType,
        pub short_name_fragments: Option<ShortNameFragments1688>,
        pub long_name: Option<MultilanguageLongNameType>,
    }
    pub struct Other {
        pub value: Vec<String>,
    }
}
"#;

    #[test]
    fn extracts_fields() {
        let index = build_index(SAMPLE.as_bytes()).unwrap();
        let json = index.to_json();
        let types = json["types"].as_object().unwrap();
        assert!(types.contains_key("ar::ArPackageTypeContent"));
        let fields = types["ar::ArPackageTypeContent"].as_object().unwrap();
        assert_eq!(
            fields["short_name_fragments"],
            Value::String("Option<ShortNameFragments1688>".to_string())
        );
        assert_eq!(
            fields["long_name"],
            Value::String("Option<MultilanguageLongNameType>".to_string())
        );
        let usage = json["field_usage"].as_object().unwrap();
        assert!(usage.contains_key("short_name_fragments"));
    }
}
