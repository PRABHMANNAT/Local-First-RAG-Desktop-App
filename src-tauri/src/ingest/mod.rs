//! Ingestion: turning sources into embedded, retrievable chunks.
//!
//! The chunker (this milestone) is the first stage. Source walkers, the
//! backpressured queue, and the indexer land across M1–M4.

pub mod chunker;
pub mod pipeline;
pub mod sources;
