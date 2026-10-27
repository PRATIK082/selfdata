# Data format strategy for the automotive ecosystem

YAML is great for human-reviewed templates, but large or AI-driven workloads
benefit from other representations. Recommended split:

| Concern | Format / Store | Why |
| --- | --- | --- |
| Human templates | YAML / TOML | Diff-friendly, readable, editable by system engineers |
| Machine exchange | JSON | Native for AI tools, web services and ARXML tooling |
| Large tabular data | Parquet / CSV | Columnar, compressed, fast for analytics |
| Local queryable store | SQLite | Zero-config, transactional, embeddable in Rust via rusqlite |
| Analytics / feature store | DuckDB | Fast SQL over Parquet/CSV, great for AI feature pipelines |
| Relationships / traceability | Graph DB (Neo4j / FalkorDB) | Requirements -> design -> code -> tests -> ECU links |
| Semantic / knowledge | RDF + OWL | Ontologies for automotive domain reasoning |
| AI context / retrieval | Vector DB (Chroma / Weaviate / Qdrant) | Embeddings for RAG over requirements and docs |
| High-performance binary | Protobuf / FlatBuffers | Compact, schema-evolvable, ideal for ECU data |

## AI-agentic (A-Z) guidance

- Keep YAML/JSON as the **authoring** layer (human + LLM editable).
- Mirror every artifact into SQLite/DuckDB for querying and traceability.
- Store embeddings of requirements/design/test text in a vector DB for RAG.
- Use a graph DB for cross-artifact traceability (requirement -> component -> runnable -> test -> ECU).
- Use Parquet for large simulation/integration datasets.
- Convert validated YAML -> ARXML via the (draft) `tools/yaml2arxml` converter.