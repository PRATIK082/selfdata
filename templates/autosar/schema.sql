-- Relational mirror of the automotive artifact graph (SQLite/DuckDB compatible)
CREATE TABLE IF NOT EXISTS requirements (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT
);

CREATE TABLE IF NOT EXISTS components (
    name TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    ecu TEXT
);

CREATE TABLE IF NOT EXISTS tests (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    requirement_id TEXT,
    FOREIGN KEY (requirement_id) REFERENCES requirements(id)
);

CREATE TABLE IF NOT EXISTS traceability (
    from_type TEXT NOT NULL,   -- requirement/component/test/ecu
    from_id   TEXT NOT NULL,
    to_type   TEXT NOT NULL,
    to_id     TEXT NOT NULL,
    link_type TEXT NOT NULL     -- satisfies/verifies/implements/allocated_to
);