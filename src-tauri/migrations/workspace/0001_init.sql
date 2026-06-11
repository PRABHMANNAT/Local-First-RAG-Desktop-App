-- Per-workspace schema. This is the source of truth; LanceDB (vectors) and the
-- tantivy index (BM25) are rebuildable projections keyed on chunk.id.
-- Foreign keys are enforced at the connection level (see db::open_pool).

CREATE TABLE source (
    id              TEXT PRIMARY KEY,
    kind            TEXT NOT NULL CHECK (kind IN ('folder','repo','youtube','pdf','url')),
    uri             TEXT NOT NULL,
    status          TEXT NOT NULL CHECK (status IN ('queued','ingesting','ready','error','stale')),
    ingested_at     INTEGER,
    last_synced_at  INTEGER,
    meta            TEXT NOT NULL DEFAULT '{}'   -- source-specific JSON
);

CREATE TABLE document (
    id            TEXT PRIMARY KEY,
    source_id     TEXT NOT NULL REFERENCES source(id) ON DELETE CASCADE,
    path_or_url   TEXT NOT NULL,
    title         TEXT,
    mime          TEXT,
    byte_size     INTEGER,
    content_hash  TEXT NOT NULL,               -- blake3; drives idempotent re-ingest
    page_count    INTEGER,
    ingested_at   INTEGER
);

CREATE TABLE chunk (
    id               TEXT PRIMARY KEY,
    document_id      TEXT NOT NULL REFERENCES document(id) ON DELETE CASCADE,
    ordinal          INTEGER NOT NULL,
    text             TEXT NOT NULL,
    token_count      INTEGER NOT NULL,
    structural_path  TEXT,                      -- "H1 > H2 > section" / "fn foo()" / "00:12-00:48"
    locator          TEXT NOT NULL              -- discriminated-union JSON (see Locator type)
);

CREATE TABLE embedding (
    chunk_id  TEXT PRIMARY KEY REFERENCES chunk(id) ON DELETE CASCADE,
    model_id  TEXT NOT NULL,                    -- e.g. "nomic-embed-text"
    dim       INTEGER NOT NULL
    -- the vector itself lives in LanceDB keyed by chunk_id; this row records
    -- provenance so a model change is detectable and re-embed stays explicit.
);

CREATE TABLE conversation (
    id          TEXT PRIMARY KEY,
    title       TEXT,
    created_at  INTEGER NOT NULL
);

CREATE TABLE message (
    id               TEXT PRIMARY KEY,
    conversation_id  TEXT NOT NULL REFERENCES conversation(id) ON DELETE CASCADE,
    role             TEXT NOT NULL CHECK (role IN ('user','assistant','system')),
    content          TEXT NOT NULL,
    created_at       INTEGER NOT NULL
);

CREATE TABLE citation (
    message_id       TEXT NOT NULL REFERENCES message(id) ON DELETE CASCADE,
    chunk_id         TEXT NOT NULL REFERENCES chunk(id),
    retrieved_score  REAL NOT NULL,
    used_in_answer   INTEGER NOT NULL,          -- bool: did the model actually cite it
    PRIMARY KEY (message_id, chunk_id)
);

-- Hot-path indexes.
CREATE INDEX idx_document_source       ON document(source_id);
CREATE INDEX idx_document_hash         ON document(content_hash);
CREATE INDEX idx_chunk_document        ON chunk(document_id);
CREATE INDEX idx_chunk_doc_ordinal     ON chunk(document_id, ordinal);
CREATE INDEX idx_message_conversation  ON message(conversation_id);
CREATE INDEX idx_citation_chunk        ON citation(chunk_id);
