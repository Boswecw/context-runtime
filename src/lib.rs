//! context-runtime — the PCC-conforming context runtime for the self-healing loop.
//!
//! Gathers candidate sources for a target file, runs the real
//! `precomputed_context_core::assemble_context` to produce a governed
//! `ContextBundleManifest`, builds code-native PCC contract payloads behind each
//! admitted ref, and serves both over HTTP for forgeHQ to consume.
//!
//! Boundary: precomputed-context-core is a Library/Contract crate that, by
//! design, owns no service runtime. This crate is that runtime — it depends on
//! PCC's real contracts and never redefines them.

pub mod assemble;
pub mod config;
pub mod error;
pub mod gather;
pub mod http;
pub mod payload;
pub mod store;

pub use precomputed_context_core as pcc;
