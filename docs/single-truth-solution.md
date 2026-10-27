# Single truth solution for the automotive domain

Goal: one authoritative, machine-readable representation of the complete
automotive system, from requirements to ECU configuration, usable by humans,
AI agents and generators.

## Layers

1. Authoring layer (human + AI): YAML/JSON/TOML templates
2. Query layer: SQLite/DuckDB + graph DB for traceability
3. AI layer: embeddings in a vector DB for RAG
4. Generation layer: YAML -> ARXML -> Rust/C code
5. Validation layer: schema validation + semantic checks

## Artifact chain

requirements -> system design -> software components -> runnables ->
port interfaces -> network frames/PDUs/signals -> ECU allocation ->
variants -> integration tests -> ARXML -> code

## Principles

- Text-first, diff-friendly, Git-versioned
- Every artifact has a stable ID and traceability links
- One source of truth, many derived views (ARXML, code, docs, tests)
- AI agents read/write the same templates and keep them consistent