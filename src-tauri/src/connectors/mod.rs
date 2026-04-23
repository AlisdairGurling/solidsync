//! PKM connectors: the pluggable modules that extract data from specific PKM
//! tools (Obsidian, Logseq, Notion, Roam) and hand it to the triplification
//! pipeline.
//!
//! The trait surface is intentionally minimal while we only have one
//! connector. It will grow as we add the second.

pub mod obsidian;
