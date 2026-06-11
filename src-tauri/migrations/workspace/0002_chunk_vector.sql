-- Vector projection table. Holds the embedding vector bytes keyed by chunk_id,
-- a rebuildable projection of the chunks (the `embedding` table records model
-- provenance; this holds the data the retriever searches). For M1 this backs a
-- brute-force cosine search; the LanceDB adapter (M2) reads the same logical
-- data and can be rebuilt from chunks + the configured embedder.

CREATE TABLE chunk_vector (
    chunk_id     TEXT PRIMARY KEY REFERENCES chunk(id) ON DELETE CASCADE,
    document_id  TEXT NOT NULL REFERENCES document(id) ON DELETE CASCADE,
    dim          INTEGER NOT NULL,
    data         BLOB NOT NULL          -- dim little-endian f32 values
);

CREATE INDEX idx_chunk_vector_document ON chunk_vector(document_id);
