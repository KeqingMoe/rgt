//! Immutable red/green syntax trees with user-defined payloads.
//!
//! `rgt` provides the tree infrastructure around a parser, not the parser
//! itself. Green trees store immutable syntax structure and text widths; red
//! trees add parent links, child indexes, and offsets for navigation.
//!
//! The [`Language`](lang::Language) trait defines the syntax kind and payload
//! types. Payloads are intentionally unconstrained so a language can store token
//! text, diagnostics, folded diagnostic flags, cache summaries, or any other
//! per-node data it needs.

pub mod builder;
pub mod green;
pub mod lang;
pub mod red;
