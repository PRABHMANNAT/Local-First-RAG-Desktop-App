//! Per-source ingestion contracts. Each source resolves to a stream of
//! documents that the chunker and indexer consume. M1 ships the folder source;
//! repo/url (M3) and youtube (M4) follow the same shape.

pub mod folder;
pub mod ignore_rules;
pub mod repo;
pub mod url;
