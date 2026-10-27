//! mfd-core — il motore di Manto.FreeDisk (vedi docs/ARCHITECTURE.md).
//!
//! Tutto cio' che capisce il filesystem vive in questo crate: walker
//! parallelo, albero di ripartizione per profondita', filtri sui nomi dei
//! file, raccolta dei warning. Il motore NON formatta: restituisce numeri e
//! struttura — la forma (griglia, righe di totali, JSON, pannelli) e' dei
//! frontend (mfd-cli, mfd-tui).
//!
//! Punti architetturali del walker (ereditati dal bench di scelta dello
//! stack, fase 1):
//! - su Windows `DirEntry::metadata()` arriva dai dati di enumerazione
//!   (FindFirstFile/FindNextFile) SENZA stat per file;
//! - symlink e junction non vengono attraversati (niente doppi conteggi,
//!   niente cicli);
//! - logica parallela a node/mfd-scan/walker-node.mjs e
//!   net/src/Manto.FreeDisk.Cli (gemello C#): i totali coincidono.

pub mod filter;
pub mod json;
pub mod lang;
pub mod scan;
pub mod tree;
pub mod view;
pub mod warning;

pub use filter::Filter;
pub use lang::{Catalog, Lang};
pub use scan::{scan, scan_with_progress, Progress, ScanOptions, ScanStats};
pub use tree::{Leaf, NodeMetrics, Tree};
pub use view::{Order, Row, RowMetrics};
pub use warning::{Warning, WarningKind, WarningLog};
