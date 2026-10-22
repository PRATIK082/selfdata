# selfdata — AUTOSAR ARXML tooling in Rust

> **Education / learning project: Rust + AUTOSAR.**
> The goal is to learn how to turn an official AUTOSAR XSD into usable Rust
> tooling — a generated model, a schema independent reader/writer/validator and
> a generic editing layer — and to document the pitfalls along the way
> (huge schemas, identifier collisions, namespace handling, …).

Tooling to **read**, **write** and **validate** AUTOSAR ARXML documents, driven by
the official AUTOSAR XSD schemas.

The Rust code for the schema is **generated** with the
[`xsd-parser`](https://crates.io/crates/xsd-parser) library from the XSD file.
The generated model is plain Rust (no `xsd-parser` dependency at runtime), so it
can be vendored and used as a standalone base library.

> The generated model (`crates/arxml-model-r4-0/src/generated.rs`, ~130 MB) is
> **not committed** — it is a build artifact. Run `./generate.ps1` once to
> create it before building the model crates.

```text
AUTOSAR_00047.xsd ──► tools/xsdgen ──► crates/arxml-model-r4-0/src/generated.rs
                                                  │
                       crates/arxml-core ◄─────────┤  (runtime: read / write / validate)
                       ├─ dom: generic DOM, path access, generic validation
                       └─ typed helpers over the generated model
                                                  │
                       crates/arxml (facade) ◄────┘  (version detection + dispatch)

crates/arxml-model-r4-0/src/generated.rs ──► tools/arxml-index ──► model-index.json
any .arxml ──► tools/arxml-cli (get / set / validate, schema independent)
```

## Workspace layout

| Path | Purpose | Depends on `xsd-parser`? |
| --- | --- | --- |
| `tools/xsdgen` | XSD → Rust code generator (CLI) | **yes** (build time only) |
| `crates/arxml-core` | Runtime helpers: read / write / validate, namespace detection | no |
| `crates/arxml-model-r4-0` | Generated model for AUTOSAR R4.0 (`AUTOSAR_00047.xsd`) + version API (`generated.rs` is produced by the generator, not committed) | no |
| `crates/arxml` | Version aware facade (`ArxmlVersion`, `detect_version`, `Model`) | no |
| `tools/arxml-index` | Scans the generated code → JSON index of types/fields | no |
| `tools/arxml-cli` | Schema independent CLI: `version`/`get`/`set`/`validate` | no |
| `schemas/` | XSD sources (`AUTOSAR_00047.xsd`, `xml.xsd`) | – |
| `generate.ps1` | One command regeneration of a model crate | – |

Only `xsd-parser-types`, `quick-xml` and `regex` are needed at runtime — the
generator itself is never linked into the shipped library.

## 1. Generate the model

```powershell
./generate.ps1
# equivalent to:
cargo run -p xsdgen --release -- `
    schemas/AUTOSAR_00047.xsd `
    crates/arxml-model-r4-0/src/generated.rs `
    --namespace http://autosar.org/schema/r4.0
```

Options of `tools/xsdgen`:

```text
xsdgen <schema.xsd> <generated.rs>
        [--namespace <ns>]        namespace of the root element (default: AUTOSAR R4.0)
        [--namespace-modules]     one Rust module per XML namespace (default)
        [--no-namespace-modules]
        [--schema-modules]        one Rust module per schema
        [--no-schema-modules]
        [--debug]                 write parser/interpreter/optimizer logs to target/
```

The generated file is huge (≈ 130 MB of Rust, ~14 600 structs) because the
AUTOSAR schema contains several thousand types.

## 2. Read / write / validate

### Version specific API

```rust
use arxml_model_r4_0::api;
use std::io::BufReader;

let file = std::fs::File::open("model.arxml")?;
let model = api::read(BufReader::new(file))?;   // streaming, event based

api::validate(&model)?;                          // strict schema validation
let xml = api::to_string(&model)?;               // serialize
api::write(&model, std::fs::File::create("out.arxml")?)?;
```

### Version aware API

```rust
use arxml::{detect_version, Model};

let data = std::fs::read("model.arxml")?;
assert_eq!(detect_version(&data).map(|v| v.label()), Some("R4.0"));

let model = Model::read(&data)?;
model.validate()?;
let xml = model.to_string()?;
```

### Runnable example

```powershell
cargo run -p arxml --example read_write_validate
```

The example (`crates/arxml/examples/read_write_validate.rs`) reads
`crates/arxml/examples/data/sample.arxml`, validates it, renames the package,
writes it back, re-reads it and validates it again.

## 3. Generic, schema independent tooling

The generated model repeats the same field groups (`short_name_fragments`,
`long_name`, `desc`, `category`, `admin_data`, `introduction`, `annotations`, …)
in thousands of structs, so no hand written code should touch those files.
Two independent tools solve that:

### `arxml_core::dom` — generic ARXML DOM (no generated code involved)

`crates/arxml-core/src/dom.rs` implements a small XML tree on top of
`quick-xml` only. It works for **any** AUTOSAR release and needs neither the
generated model nor `xsd-parser`:

```rust
use arxml_core::dom::Document;
use std::io::BufReader;

let file = std::fs::File::open("model.arxml")?;
let mut doc = Document::parse(BufReader::new(file))?;      // streaming parse

// query by path: NAME, NAME[n] (1-based), * (wildcard)
let names = doc.texts_at("AR-PACKAGES/AR-PACKAGE/SHORT-NAME");

// update by path
doc.set_text_at("AR-PACKAGES/AR-PACKAGE/SHORT-NAME", "NewName");

// generic validation (root element, namespace, non empty SHORT-NAME,
// unique UUIDs, no mixed content)
for issue in doc.validate() {
    eprintln!("{:?} at {} – {}", issue.kind, issue.path, issue.message);
}

std::fs::write("out.arxml", doc.to_xml_string()?)?;
```

Runnable example (works without the 130 MB model crate):

```powershell
cargo run -p arxml-core --example generic_edit
```

### `tools/arxml-cli` — same capabilities from the command line

```powershell
cargo run -p arxml-cli -- version  model.arxml
cargo run -p arxml-cli -- get      model.arxml "AR-PACKAGES/AR-PACKAGE/SHORT-NAME"
cargo run -p arxml-cli -- set      model.arxml "AR-PACKAGES/AR-PACKAGE/SHORT-NAME" NewName -o out.arxml
cargo run -p arxml-cli -- validate model.arxml
```

### `tools/arxml-index` — index of the generated code

The tool scans the generated Rust and writes a JSON index, so field/type
look-ups never require reading the generated file:

```powershell
cargo run -p arxml-index --release -- `
    crates/arxml-model-r4-0/src/generated.rs -o target/model-index.json
```

```json
{
  "types": { "ar::ArPackageTypeContent": { "short_name": "IdentifierType", "...": "..." } },
  "field_usage": { "short_name_fragments": ["ar::Type1", "ar::Type2"] }
}
```

For the AUTOSAR R4.0 model this yields 14 640 types and 3 011 distinct fields;
`short_name_fragments` alone appears in 779 types.

## How validation works

`arxml_core::validate_arxml` serializes the model and parses the result back
with the **strict** deserializer that `xsd-parser` generates. The generated
deserializer enforces

* required elements (`minOccurs = 1`) and cardinality (`maxOccurs`),
* enumerations and simple type facets (patterns, lengths, ranges),
* duplicate elements/attributes and unexpected elements/attributes,
* the namespaces of the document.

Errors are reported with the position and the element path of the offending
node. This is the practical equivalent of an XSD validation without shipping a
separate XSD validator.

## Adding another AUTOSAR release

1. Put the new XSD (and the W3C `xml.xsd` it imports) into `schemas/`.
2. Create a model crate, e.g. `crates/arxml-model-r4-1` with
   `arxml-core` + `xsd-parser-types` + `regex` as dependencies and a
   `generated.rs` produced by
   `cargo run -p xsdgen --release -- schemas/AUTOSAR_00047_4-1.xsd crates/arxml-model-r4-1/src/generated.rs --namespace <ns>`.
3. Add the release to `ArxmlVersion` in `crates/arxml/src/lib.rs`, add the
   feature `r4-1 = ["dep:arxml-model-r4-1"]` and extend the `Model` enum.
4. Add the crate to `[workspace] members`.

Every release keeps its own generated types, so documents of different releases
can be read, written and validated side by side; `detect_version` dispatches a
document to the right model.

## What this project demonstrates (learning notes)

1. **XSD → Rust is a pipeline, not a one-liner.** `xsd-parser` splits into
   parser → interpreter → optimizer → generator → renderer. Each stage has flags
   that decide whether simple-type restrictions survive, whether types are
   flattened, and whether namespaces become separate Rust modules.
2. **Big schemas break naive assumptions.** AUTOSAR R4.0 yields 330 complex
   types, 742 simple types and ~14 600 generated structs (~130 MB of Rust).
   The generated code compiles, but it is unreadable by hand — which is exactly
   why the generic DOM layer and the index tool exist.
3. **Name collisions are the real work.** XSD names like `MEMORY-SECTION` and
   `MEMORY-SECTION-TYPE` map to the same Rust identifier, and the XMLSchema
   built-ins (`BooleanType`, `StringType`, …) clash with AUTOSAR types. The fix
   belongs in the *naming layer* (`tools/xsdgen/src/naming.rs`), not in
   post-processing the generated file: the `Named`/`Generated` distinction of
   `xsd-parser` must be preserved, otherwise the generator produces code that
   does not compile (thousands of type-inference errors).
4. **Two access strategies are needed.** A typed model for strict validation and
   code generation, and a generic DOM for editing files whose release you do
   not have a model for.
5. **Validation without a separate XSD validator.** Serializing the model and
   parsing it back with the generated strict deserializer checks cardinality,
   enumerations, facets, duplicates and namespaces.
6. **Multiple releases = multiple model crates + one facade.** Each release gets
   its own generated crate; `ArxmlVersion`/`detect_version` dispatch documents
   to the right model.

## Notes / limitations

* The generated model is very large; compiling it takes minutes. The model
  crate has a `generated` feature (enabled by default) that can be switched off
  when only the facade is needed.
* `xsd-parser` maps some AUTOSAR types to the same Rust identifier (e.g.
  `MEMORY-SECTION` and `MEMORY-SECTION-TYPE`). `tools/xsdgen/src/naming.rs`
  installs a small naming layer that makes every type and enum variant
  identifier unique (`MemorySectionType`, `MemorySectionType2`, …) and splits the
  XML namespaces into separate Rust modules, so the built-in XMLSchema types do
  not collide with the AUTOSAR types.
* Simple type restrictions are preserved on purpose
  (`OptimizerFlags::USE_UNRESTRICTED_BASE_TYPE_SIMPLE` stays disabled) so that
  the deserializer can validate patterns and lengths.
