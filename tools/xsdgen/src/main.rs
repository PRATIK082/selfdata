use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod naming;

use anyhow::{Context, Result};
use xsd_parser::config::{
    Generate, GeneratorFlags, OptimizerFlags, ParserFlags, Resolver, Schema,
};
use xsd_parser::pipeline::renderer::NamespaceSerialization;
use xsd_parser::Config;
use xsd_parser_types::misc::Namespace;

/// Default namespace of the AUTOSAR schema (release 4.x).
const AUTOSAR_NAMESPACE: &[u8] = b"http://autosar.org/schema/r4.0";

#[derive(Debug)]
struct Options {
    schema: PathBuf,
    output: PathBuf,
    target_namespace: Vec<u8>,
    namespace_modules: bool,
    schema_modules: bool,
    debug: bool,
}

fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .try_init();

    let options = parse_args()?;
    generate(&options)
}

fn parse_args() -> Result<Options> {
    let mut schema: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut target_namespace: Option<Vec<u8>> = None;
    let mut namespace_modules = true;
    let mut schema_modules = false;
    let mut debug = false;

    let mut args = env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let arg_str = arg.to_string_lossy().into_owned();
        match arg_str.as_str() {
            "--namespace-modules" => namespace_modules = true,
            "--no-namespace-modules" => namespace_modules = false,
            "--schema-modules" => schema_modules = true,
            "--no-schema-modules" => schema_modules = false,
            "--debug" => debug = true,
            "--namespace" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--namespace requires a value"))?;
                target_namespace = Some(value.to_string_lossy().into_owned().into_bytes());
            }
            "--help" | "-h" => {
                usage();
            }
            _ => {
                if schema.is_none() {
                    schema = Some(PathBuf::from(arg));
                } else if output.is_none() {
                    output = Some(PathBuf::from(arg));
                } else {
                    anyhow::bail!("unexpected argument: {arg_str}");
                }
            }
        }
    }

    Ok(Options {
        schema: schema.ok_or_else(|| anyhow::anyhow!("missing <schema.xsd>"))?,
        output: output.ok_or_else(|| anyhow::anyhow!("missing <generated.rs>"))?,
        target_namespace: target_namespace.unwrap_or_else(|| AUTOSAR_NAMESPACE.to_vec()),
        namespace_modules,
        schema_modules,
        debug,
    })
}

fn usage() -> ! {
    eprintln!(
        "usage: xsdgen <schema.xsd> <generated.rs> [--namespace <ns>] [--namespace-modules] \
         [--schema-modules] [--debug]"
    );
    std::process::exit(2);
}

fn generate(options: &Options) -> Result<()> {
    let input = options
        .schema
        .canonicalize()
        .with_context(|| format!("cannot resolve schema path {}", options.schema.display()))?;

    let mut config = Config::default();

    config.parser.resolver = vec![Resolver::File];
    config.parser.flags = ParserFlags::all();
    config.parser.schemas = vec![Schema::File(input)];

    if options.debug {
        config.parser.debug_output = Some("target/parser.log".into());
        config.interpreter.debug_output = Some("target/interpreter.log".into());
        config.optimizer.debug_output = Some("target/optimizer.log".into());
    }

    // Keep simple-type restrictions (patterns, lengths, ranges) so the generated
    // deserializer performs facet validation.
    config.optimizer.flags = OptimizerFlags::REMOVE_EMPTY_ENUM_VARIANTS
        | OptimizerFlags::REMOVE_EMPTY_ENUMS
        | OptimizerFlags::REMOVE_DUPLICATE_UNION_VARIANTS
        | OptimizerFlags::REMOVE_EMPTY_UNIONS
        | OptimizerFlags::REMOVE_DUPLICATES
        | OptimizerFlags::MERGE_DYNAMIC_TYPES
        | OptimizerFlags::FLATTEN_COMPLEX_TYPES
        | OptimizerFlags::FLATTEN_UNIONS
        | OptimizerFlags::MERGE_ENUM_UNIONS
        | OptimizerFlags::CONVERT_DYNAMIC_TO_CHOICE;

    config.generator.generate = Generate::Named;

    // Put every XML namespace into its own Rust module. Without this the built-in
    // XMLSchema types (`BooleanType`, `StringType`, ...) end up in the same module
    // as the AUTOSAR types and collide with identically named AUTOSAR types.
    let mut generator_flags = GeneratorFlags::empty();
    if options.namespace_modules {
        generator_flags |= GeneratorFlags::USE_NAMESPACE_MODULES;
    }
    if options.schema_modules {
        generator_flags |= GeneratorFlags::USE_SCHEMA_MODULES;
    }
    config.generator.flags = generator_flags;

    config = config
        .with_naming(naming::UniqueNaming::default())
        .with_quick_xml_config(
            NamespaceSerialization::Global,
            Some(Namespace::new_const(
                Box::leak(options.target_namespace.clone().into_boxed_slice()),
            )),
            false,
        );

    let code = xsd_parser::generate(config)
        .context("xsd-parser failed to generate code")?
        .to_string();

    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("cannot create output directory {}", parent.display())
        })?;
    }

    fs::write(&options.output, &code).with_context(|| {
        format!(
            "cannot write generated code to {}",
            options.output.display()
        )
    })?;

    format_output(&options.output)?;

    eprintln!(
        "generated {} ({} bytes)",
        options.output.display(),
        fs::metadata(&options.output)?.len()
    );

    Ok(())
}

fn format_output(path: &Path) -> Result<()> {
    let result = Command::new("rustfmt")
        .args(["--edition", "2021"])
        .arg(path)
        .status();

    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => {
            eprintln!("warning: rustfmt exited with {status}; generated code is unformatted");
            Ok(())
        }
        Err(error) => {
            eprintln!("warning: could not run rustfmt: {error}; generated code is unformatted");
            Ok(())
        }
    }
}
